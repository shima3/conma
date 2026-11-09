//! __JS_call__: evaluate JavaScript in the host of the VM for wasm32
//! (consh/js), and __JS_value__: a value of the host as a value of ConSh.
//! The native VM has no JavaScript: both go to onError.

use super::stream::new_stream;
use super::{take_n, PResult, VmError};
use crate::sched::Machine;
use crate::sys;
use crate::trace::value_to_display;
use crate::value::*;
use crate::vproc::VProc;

/// __JS_call__ onError code args ,(inputStream outputStream): the host
/// evaluates the String code with args (an LList of Strings and LLists of
/// them), and writes the result to inputStream as lines of S-expressions;
/// what is written to outputStream goes to the host (the standard input of
/// the code), and it is closed when the last reference to it is dropped. See
/// 2.2 of consh/js/PLAN-ja.md.
pub fn call(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__JS_call__";
    let mut a = take_n(vp, name, 3)?;
    let args = a.pop().unwrap();
    let code = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    if value_type_name(&code) != "String" {
        vp.prim_error(on_error, &format!("{}: expected String code, got {}", name, value_type_name(&code)));
        return Ok(());
    }
    let mut json = String::new();
    if let Err(t) = args_json(&args, &mut json) {
        vp.prim_error(on_error, &format!("{}: expected an LList of Strings and LLists, got {}", name, t));
        return Ok(());
    }
    let code = value_to_display(&code).map_err(VmError::Error)?;
    match sys::js_call(&code, &json) {
        Ok((r, w)) => {
            let input = Value::Stream(new_stream(m, r, 'r'));
            let w = new_stream(m, w, 'w');
            w.drop_close.set(true);
            let output = Value::Stream(w);
            vp.prim_return(vec![input, output]);
        }
        Err(e) => vp.prim_error(on_error, &format!("{}: {}", name, sys::strerror(&e))),
    }
    Ok(())
}

/// __JS_value__ onError id ,(String): the JS value numbered id in the host
/// (a String that the host gave in a result of __JS_call__), as the
/// S-expression of a value of ConSh: (s "text"), (l (...)), (n) or (f) for a
/// function. The host makes it when it is asked, so a change of an object is
/// seen. See 2.5 of consh/js/PLAN-ja.md.
pub fn value(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__JS_value__";
    let mut a = take_n(vp, name, 2)?;
    let id = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    if value_type_name(&id) != "String" {
        vp.prim_error(on_error, &format!("{}: expected String id, got {}", name, value_type_name(&id)));
        return Ok(());
    }
    let id = value_to_display(&id).map_err(VmError::Error)?;
    match sys::js_value(&id) {
        Ok(text) => vp.prim_return(vec![string_value(&text)]),
        Err(e) => vp.prim_error(on_error, &format!("{}: {}", name, sys::strerror(&e))),
    }
    Ok(())
}

/// The LList v (null or an LList) as a JSON array of strings and arrays;
/// Err with the type name of the first value that is neither.
fn args_json(v: &Value, out: &mut String) -> Result<(), String> {
    out.push('[');
    let mut cur = v.clone();
    let mut first = true;
    loop {
        let cell = match &cur {
            Value::Null => break,
            Value::LList(c) => c.clone(),
            other => return Err(value_type_name(other).to_string()),
        };
        if !first {
            out.push(',');
        }
        first = false;
        match &cell.head {
            h @ (Value::Null | Value::LList(_)) => args_json(h, out)?,
            h if value_type_name(h) == "String" => {
                let s = value_to_display(h).map_err(|_| "String".to_string())?;
                json_string(&s, out);
            }
            h => return Err(value_type_name(h).to_string()),
        }
        cur = cell.tail.clone();
    }
    out.push(']');
    Ok(())
}

fn json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
