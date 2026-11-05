//! VProc primitives, and loading a program for __VProc_spawn__.

use super::{err, take_n, PResult};
use crate::module::{build_name_index, load_module, GVEnv};
use crate::sched::Machine;
use crate::sexp::{self, Sexp};
use crate::trace::value_to_display;
use crate::value::*;
use crate::vproc::{State, VProc, VProcRef};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;

fn current(m: &Machine) -> VProcRef {
    m.current.clone().expect("no current VProc")
}

/// One operand that must be a VProc ("missing argument" if there is none).
fn vproc_arg(vp: &mut VProc, name: &str) -> Result<VProcRef, super::VmError> {
    if vp.olist.is_empty() {
        return err(format!("{}: missing argument", name));
    }
    match vp.olist.remove(0) {
        Value::VProc(t) => Ok(t),
        other => err(format!("{}: expected VProc, got {}", name, value_type_name(&other))),
    }
}

pub fn current_vproc(m: &mut Machine, vp: &mut VProc) -> PResult {
    let cur = current(m);
    vp.prim_return(vec![Value::VProc(cur)]);
    Ok(())
}

pub fn suspend(m: &mut Machine, vp: &mut VProc) -> PResult {
    let target = vproc_arg(vp, "__VProc_suspend__")?;
    vp.prim_return(vec![]);
    m.suspend(&target);
    Ok(())
}

pub fn resume(m: &mut Machine, vp: &mut VProc) -> PResult {
    let target = vproc_arg(vp, "__VProc_resume__")?;
    vp.prim_return(vec![]);
    m.resume(&target);
    Ok(())
}

pub fn wait(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__VProc_wait__";
    let mut a = take_n(vp, name, 2)?;
    let target = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    let target = match target {
        Value::VProc(t) => t,
        other => {
            vp.prim_error(on_error, &format!("{}: expected VProc, got {}", name, value_type_name(&other)));
            return Ok(());
        }
    };
    let cur = current(m);
    if Rc::ptr_eq(&target, &cur) {
        vp.prim_error(on_error, &format!("{}: a VProc cannot wait for itself", name));
        return Ok(());
    }
    vp.prim_return(vec![]);
    if target.state.get() != State::Finished {
        m.join(&cur, &target);
    }
    Ok(())
}

pub fn id(_m: &mut Machine, vp: &mut VProc) -> PResult {
    let target = vproc_arg(vp, "__VProc_id__")?;
    vp.prim_return(vec![num(target.serial as f64)]);
    Ok(())
}

pub fn kill(m: &mut Machine, vp: &mut VProc) -> PResult {
    let target = vproc_arg(vp, "__VProc_kill__")?;
    vp.prim_return(vec![]);
    m.kill(vp, &target);
    Ok(())
}

/// __VProc_start__ Closure ,(VProc)
pub fn start(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__VProc_start__";
    if vp.olist.is_empty() {
        return err(format!("{}: missing argument", name));
    }
    let func = vp.olist.remove(0);
    let gvenv = match &func {
        Value::Closure(c) => c.gvenv.clone().unwrap_or_else(|| vp.gvenv.clone()),
        other => return err(format!("{}: expected Closure, got {}", name, value_type_name(other))),
    };
    let child = m.make_vproc(func, Vec::new(), None, gvenv, vp.cwd.clone());
    child.vp.borrow_mut().gvenv_files = vp.gvenv_files.clone();
    m.add(child.clone());
    vp.prim_return(vec![Value::VProc(child)]);
    Ok(())
}

/// __VProc_spawn__ onError entryPoint sourceFileName errorSink argumentSeq inputSeq outputSink ,(VProc)
pub fn spawn(m: &mut Machine, vp: &mut VProc) -> PResult {
    let name = "__VProc_spawn__";
    let mut a = take_n(vp, name, 7)?;
    let sinks = a.split_off(3);
    let source = a.pop().unwrap();
    let entry = a.pop().unwrap();
    let on_error = a.pop().unwrap();
    for (what, val) in [("entryPoint", &entry), ("sourceFileName", &source)] {
        if value_type_name(val) != "String" {
            let msg = format!("{}: expected String {}, got {}", name, what, value_type_name(val));
            vp.prim_error(on_error, &msg);
            return Ok(());
        }
    }
    let entry = value_to_display(&entry).map_err(super::VmError::Error)?;
    let source = value_to_display(&source).map_err(super::VmError::Error)?;
    let (module, gvenv) = match load_program(&source, &vp.cwd) {
        Ok(x) => x,
        Err(e) => {
            vp.prim_error(on_error, &format!("{}: {}", name, e));
            return Ok(());
        }
    };
    let func = build_name_index(&module).get(&entry).and_then(|g| gvenv.get(*g));
    let func = match func {
        Some(f) if !f.is_null() => f,
        _ => {
            vp.prim_error(on_error, &format!("{}: entry point not defined: {}", name, entry));
            return Ok(());
        }
    };
    let child = m.make_vproc(func, sinks, Some(&source), gvenv, vp.cwd.clone());
    m.add(child.clone());
    vp.prim_return(vec![Value::VProc(child)]);
    Ok(())
}

/// The repository: the directory above vm/target/release/conma.
fn root_dir() -> PathBuf {
    let exe = std::env::current_exe().and_then(|p| p.canonicalize()).unwrap_or_default();
    exe.ancestors().nth(4).map(Path::to_path_buf).unwrap_or_default()
}

/// Run bin/includer and bin/resolver on source_file (relative to cwd) and
/// load the result, as load_program in test/vproc_step.py.
pub fn load_program(source_file: &str, cwd: &str) -> Result<(Sexp, Rc<GVEnv>), String> {
    if !Path::new(cwd).join(source_file).is_file() {
        return Err(format!("file not found: {}", source_file));
    }
    let root = root_dir();
    let inc = Command::new(root.join("bin/includer"))
        .arg("--include")
        .arg(root.join("include"))
        .arg(source_file)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("cannot run includer: {}", crate::sys::errno_str(&e)))?;
    if !inc.status.success() {
        return Err(format!("includer failed: {}", String::from_utf8_lossy(&inc.stderr).trim()));
    }
    // File names in SInfo relative to the directory above the repository.
    let parent = root.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    let ast = String::from_utf8_lossy(&inc.stdout).replace(&format!("\"{}/", parent), "\"");
    let mut child = Command::new(root.join("bin/resolver"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run resolver: {}", crate::sys::errno_str(&e)))?;
    let mut stdin = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(ast.as_bytes());
    });
    let res = child.wait_with_output().map_err(|e| format!("cannot run resolver: {}", crate::sys::errno_str(&e)))?;
    let _ = writer.join();
    if !res.status.success() {
        return Err(format!("resolver failed: {}", String::from_utf8_lossy(&res.stderr).trim()));
    }
    let module = sexp::parse(&String::from_utf8_lossy(&res.stdout))?;
    let gvenv = load_module(&module)?;
    Ok((module, gvenv))
}
