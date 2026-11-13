//! VProc primitives, and loading a program for __VProc_spawn__.

use super::{err, take_n, PResult};
use crate::module::{build_name_index, load_module, GVEnv};
use crate::sched::Machine;
use crate::sexp::{self, Sexp};
use crate::trace::value_to_display;
use crate::value::*;
use crate::vproc::{State, VProc, VProcRef};
use std::path::{Path, PathBuf};
#[cfg(unix)]
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
pub fn root_dir() -> PathBuf {
    let exe = std::env::current_exe().and_then(|p| p.canonicalize()).unwrap_or_default();
    exe.ancestors().nth(4).map(Path::to_path_buf).unwrap_or_default()
}

/// The directory of the standard library (include/): CONMA_INCLUDE, or the
/// include/ of the repository the VM is in.
pub fn include_dir() -> PathBuf {
    match std::env::var_os("CONMA_INCLUDE") {
        Some(d) => PathBuf::from(d),
        None => root_dir().join("include"),
    }
}

thread_local! {
    /// True when the program runs a module of concise ConMa (without (Uses ...)):
    /// the modules __VProc_spawn__ makes are then of concise ConMa too, as the
    /// Python VM makes them, so that the steps and traces are the same.
    static CONCISE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Make the modules of __VProc_spawn__ of concise ConMa (main.rs, for a module
/// without (Uses ...)).
pub fn set_concise(concise: bool) {
    CONCISE.with(|c| c.set(concise));
}

/// True if the modules of __VProc_spawn__ are of concise ConMa (set_concise).
#[allow(dead_code)]
pub fn is_concise() -> bool {
    CONCISE.with(|c| c.get())
}

/// The include directories of the standard library: for fast ConMa include/fast
/// (if it is there), then include/; for concise ConMa (set_concise) include/ref,
/// then include/ (docs/FAST-PLAN-ja.md 4.6).
pub fn include_args() -> Vec<String> {
    let inc = include_dir();
    let first = inc.join(if CONCISE.with(|c| c.get()) { "ref" } else { "fast" });
    let mut a = Vec::new();
    if first.is_dir() {
        a.push("--include".to_string());
        a.push(first.to_string_lossy().into_owned());
    }
    a.push("--include".to_string());
    a.push(inc.to_string_lossy().into_owned());
    a
}

/// Make the module of source_file (relative to cwd) with the ConMa frontend
/// that this VM embeds (conma build, in another process of this executable),
/// and load it, as load_program in test/vproc_step.py does with bin/includer
/// and bin/resolver. On an error, the first line of what the frontend wrote.
pub fn load_program(source_file: &str, cwd: &str) -> Result<(Sexp, Rc<GVEnv>), String> {
    if !is_file(&Path::new(cwd).join(source_file)) {
        return Err(format!("file not found: {}", source_file));
    }
    let text = build_module(source_file, cwd)?;
    let module = sexp::parse(&text)?;
    let gvenv = load_module(&module)?;
    Ok((module, gvenv))
}

#[cfg(unix)]
fn is_file(path: &Path) -> bool {
    path.is_file()
}

/// The VM for wasm32 has no std::fs: ask its virtual OS (or the host's files).
#[cfg(not(unix))]
fn is_file(path: &Path) -> bool {
    matches!(crate::sys::is_dir(&path.to_string_lossy()), Ok(false))
}

/// The module text of source_file (relative to cwd), from conma build in
/// another process of this executable, or the first line of its error.
#[cfg(unix)]
fn build_module(source_file: &str, cwd: &str) -> Result<String, String> {
    let root = root_dir();
    let exe = std::env::current_exe().map_err(|e| format!("cannot run the frontend: {}", crate::sys::errno_str(&e)))?;
    let res = Command::new(exe)
        .arg("build")
        .args(include_args())
        .arg(source_file)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .env_remove("CONMA_TRANSPILED_REPORT")
        .env_remove("CONMA_PROFILE")
        .env_remove("CONMA_TRANSPILED_COUNTS")
        .output()
        .map_err(|e| format!("cannot run the frontend: {}", crate::sys::errno_str(&e)))?;
    if !res.status.success() {
        let err = String::from_utf8_lossy(&res.stderr);
        let first = err.trim().lines().next().unwrap_or("").trim().to_string();
        return Err(format!("includer failed: {}", first));
    }
    // File names in SInfo relative to the directory above the repository.
    let parent = root.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(String::from_utf8_lossy(&res.stdout).replace(&format!("\"{}/", parent), "\""))
}

/// The VM for wasm32: the host runs the ConMa frontend as conma build does
/// (consh/js), and gives the module text with the file names in SInfo
/// relative to the directory above the repository, or the first line of its
/// error.
#[cfg(not(unix))]
fn build_module(source_file: &str, cwd: &str) -> Result<String, String> {
    crate::sys::build_module(source_file, cwd).map_err(|e| format!("includer failed: {}", e))
}
