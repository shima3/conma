//! The table of primitives. A primitive is looked up by its ConMa name for a
//! global variable without a definition, and by the name of its Python
//! function for `(PrimFunc "...")`; traces write the Python name, so that they
//! match test/vproc_step.py.

use crate::sched::Machine;
use crate::vproc::VProc;

mod basic;
mod llist;
pub mod os;

pub enum VmError {
    /// An error that finishes the VProc (VProcError).
    Error(String),
    /// The interpreter exits with this code.
    Exit(i32),
}

pub type PResult = Result<(), VmError>;

pub fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::Error(msg.into()))
}

pub struct PrimDef {
    pub name: &'static str,
    pub pyname: &'static str,
    pub f: fn(&mut Machine, &mut VProc) -> PResult,
}

macro_rules! prims {
    ($( $name:literal => $py:literal, $f:path; )*) => {
        static PRIMS: &[PrimDef] = &[ $( PrimDef { name: $name, pyname: $py, f: $f }, )* ];
    };
}

prims! {
    "__is_null__"          => "prim_is_null", basic::is_null;
    "__type_name__"        => "prim_type_name", basic::type_name;
    "__OS_exit_normal__"   => "prim_OS_exit_normal", os::exit_normal;
    "__OS_exit_error__"    => "prim_OS_exit_error", os::exit_error;
    "__OS_print__"         => "prim_OS_print", os::print;
    "__OS_print_error__"   => "prim_OS_print_error", os::print_error;
    "__LList_cons__"       => "prim_LList_cons", llist::cons;
    "__LList_uncons__"     => "prim_LList_uncons", llist::uncons;
}

pub fn lookup(name: &str) -> Option<&'static PrimDef> {
    PRIMS.iter().find(|p| p.name == name)
}

pub fn lookup_pyname(pyname: &str) -> Option<&'static PrimDef> {
    PRIMS.iter().find(|p| p.pyname == pyname)
}

/// Take the first n operands, or fail with "<name>: missing argument(s)"
/// ("missing argument" for one).
pub fn take_args(vp: &mut VProc, name: &str, n: usize) -> Result<Vec<crate::value::Value>, VmError> {
    if vp.olist.len() < n {
        return if n == 1 {
            err(format!("{}: missing argument", name))
        } else {
            err(format!("{}: missing argument(s)", name))
        };
    }
    Ok(vp.olist.drain(..n).collect())
}
