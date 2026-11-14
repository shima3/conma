//! __Wasm_compile__ and __Wasm_call__, the primitives of the feature "wasm"
//! of fast ConMa (consh/SJS-WASM-PLAN-ja.md 5.3, docs/PRIMITIVES.md 16): a
//! module of functions on numbers, written in a subset of the WebAssembly
//! text format (wasm_asm.rs), run by the host of the VM for wasm32 (consh/js).
//! The native VM assembles the text and then goes to onError, as it has no
//! WebAssembly.

use super::wasm_asm;
use super::{take_n, PResult, VmError};
use crate::sched::Machine;
use crate::sys;
use crate::trace::value_to_display;
use crate::value::*;
use crate::vproc::VProc;
use std::rc::Rc;

/// The most fuel a call can be given (the integers of an f64 are exact up to 2^53).
const MAX_FUEL: f64 = 9007199254740992.0;

/// WebAssembly can be used: the VM for wasm32, without --trace and CONMA_PROFILE (which count the
/// steps of ConMa) and without CONMA_NO_WASM.
fn available(m: &Machine) -> bool {
    sys::wasm_available()
        && !m.trace
        && m.profile.is_none()
        && !sys::env_vars().iter().any(|(k, v)| k == "CONMA_NO_WASM" && !v.is_empty())
}

/// __Wasm_compile__ onError wat ,(module): the module of the String wat.
pub fn compile(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__Wasm_compile__";
    let mut a = take_n(vp, name, 2)?;
    let wat = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    if value_type_name(&wat) != "String" {
        vp.prim_error(on_error, &format!("{}: expected String, got {}", name, value_type_name(&wat)));
        return Ok(());
    }
    let text = value_to_display(&wat).map_err(VmError::Error)?;
    let asm = match wasm_asm::assemble(&text) {
        Ok(asm) => asm,
        Err(e) => {
            vp.prim_error(on_error, &format!("{}: {}", name, e));
            return Ok(());
        }
    };
    if !available(m) {
        vp.prim_error(on_error, &format!("{}: WebAssembly is not available", name));
        return Ok(());
    }
    match sys::wasm_compile(&asm.bytes) {
        Ok(host) => {
            m.serials.wasms += 1;
            let w = WasmObj { serial: m.serials.wasms, host, exports: asm.exports, memory: asm.memory, memory_max: asm.memory_max };
            vp.prim_return(vec![Value::Wasm(Rc::new(w))]);
        }
        Err(e) => vp.prim_error(on_error, &format!("{}: {}", name, e)),
    }
    Ok(())
}

/// __Wasm_call__ onError module name args fuel ,(status value): call the exported function name
/// with args (an LList of Numbers). status is "ok" with the result, "fuel" with null when the fuel
/// ran out (the time slice ends, so that the host sees its events before the program calls again),
/// or "trap" with the message.
pub fn call(m: &mut Machine, vp: &mut VProc) -> PResult {
    let pname = "__Wasm_call__";
    let mut a = take_n(vp, pname, 5)?;
    let fuel = a.pop().unwrap();
    let args = a.pop().unwrap();
    let fname = a.pop().unwrap();
    let module = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    call_with(m, vp, pname, on_error, module, fname, args, None, fuel)
}

/// __Wasm_call_memory__ onError module name args memory fuel ,(status value memory): as __Wasm_call__,
/// the memory of the module (which must have one) filled with zeros and then with the Numbers of
/// the LList memory as f64 words from address 0, and, on "ok", the same number of words of the memory
/// after the call as an LList (null otherwise). consh/SJS-WASM-ARRAY-PLAN-ja.md, stage A1.
pub fn call_memory(m: &mut Machine, vp: &mut VProc) -> PResult {
    let pname = "__Wasm_call_memory__";
    let mut a = take_n(vp, pname, 6)?;
    let fuel = a.pop().unwrap();
    let memory = a.pop().unwrap();
    let args = a.pop().unwrap();
    let fname = a.pop().unwrap();
    let module = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    call_with(m, vp, pname, on_error, module, fname, args, Some(memory), fuel)
}

/// The Numbers of the LList v, or the type name of what is not one.
fn numbers(v: &Value) -> Result<Vec<f64>, String> {
    let mut nums = Vec::new();
    let mut cur = v.clone();
    loop {
        let cell = match &cur {
            Value::Null => return Ok(nums),
            Value::LList(c) => c.clone(),
            other => return Err(value_type_name(other).to_string()),
        };
        match &cell.head {
            Value::Number(x) => nums.push(*x),
            other => return Err(format!("{} in it", value_type_name(other))),
        }
        cur = cell.tail.clone();
    }
}

#[allow(clippy::too_many_arguments)]
fn call_with(m: &mut Machine, vp: &mut VProc, pname: &str, on_error: Value, module: Value, fname: Value, args: Value, memory: Option<Value>, fuel: Value) -> PResult {
    let w = match &module {
        Value::Wasm(w) => w.clone(),
        other => {
            vp.prim_error(on_error, &format!("{}: expected a Wasm module, got {}", pname, value_type_name(other)));
            return Ok(());
        }
    };
    if value_type_name(&fname) != "String" {
        vp.prim_error(on_error, &format!("{}: expected String name, got {}", pname, value_type_name(&fname)));
        return Ok(());
    }
    let fname = value_to_display(&fname).map_err(VmError::Error)?;
    let nums = match numbers(&args) {
        Ok(n) => n,
        Err(t) => {
            vp.prim_error(on_error, &format!("{}: expected an LList of Numbers, got {}", pname, t));
            return Ok(());
        }
    };
    let mut words = match &memory {
        None => Vec::new(),
        Some(mv) => {
            if !w.memory {
                vp.prim_error(on_error, &format!("{}: the module has no memory", pname));
                return Ok(());
            }
            match numbers(mv) {
                // words that do not fit in the memory at its max are an error of the arguments (as the
                // host would fail to grow it), not a trap
                Ok(n) if n.len() as u64 > w.memory_max * 8192 => {
                    vp.prim_error(on_error, &format!("{}: {} words do not fit in the memory of the module (at most {} pages, {} words)", pname, n.len(), w.memory_max, w.memory_max * 8192));
                    return Ok(());
                }
                Ok(n) => n,
                Err(t) => {
                    vp.prim_error(on_error, &format!("{}: expected an LList of Numbers for the memory, got {}", pname, t));
                    return Ok(());
                }
            }
        }
    };
    let fuel = match fuel {
        Value::Number(f) if f >= 1.0 && f <= MAX_FUEL && f == f.floor() => f,
        other => {
            let what = match &other {
                Value::Number(f) => format!("{}", number_to_str(*f)),
                v => value_type_name(v).to_string(),
            };
            vp.prim_error(on_error, &format!("{}: expected fuel, an integer from 1 to 2^53, got {}", pname, what));
            return Ok(());
        }
    };
    let arity = match w.exports.iter().find(|(n, _)| *n == fname) {
        Some((_, k)) => *k,
        None => {
            vp.prim_error(on_error, &format!("{}: no exported function \"{}\"", pname, fname));
            return Ok(());
        }
    };
    if nums.len() != arity {
        vp.prim_error(on_error, &format!("{}: \"{}\" takes {} argument(s), got {}", pname, fname, arity, nums.len()));
        return Ok(());
    }
    let with_memory = memory.is_some();
    let mut ok = false;
    let mut results = match sys::wasm_call(w.host, &fname, &nums, &mut words, fuel) {
        sys::WasmResult::Ok(x) => {
            ok = true;
            vec![string_value("ok"), Value::Number(x)]
        }
        sys::WasmResult::Fuel => {
            m.end_slice = true;
            vec![string_value("fuel"), Value::Null]
        }
        sys::WasmResult::Trap(msg) => vec![string_value("trap"), string_value(&msg)],
    };
    if with_memory {
        let mut list = Value::Null;
        if ok {
            for x in words.iter().rev() {
                list = Value::LList(Rc::new(LList { head: Value::Number(*x), tail: list }));
            }
        }
        results.push(list);
    }
    vp.prim_return(results);
    Ok(())
}
