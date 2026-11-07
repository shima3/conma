//! The table of primitives. A primitive is looked up by its ConMa name for a
//! global variable without a definition, and by the name of its Python
//! function for `(PrimFunc "...")`; traces write the Python name, so that they
//! match test/vproc_step.py.

use crate::sched::Machine;
use crate::vproc::VProc;

pub mod basic;
mod cchain;
pub mod collections;
pub mod control;
pub mod lazy;
pub mod llist;
pub mod number;
mod process;
pub mod stream;
pub mod os;
mod refs;
pub mod string;
mod vprocs;

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
    "__OS_exit__"          => "prim_OS_exit", os::exit;
    "__Number_fromString__" => "prim_Number_fromString", number::from_string;
    "__Number_toString__"  => "prim_Number_toString", number::to_string;
    "__Number_is_zero__"   => "prim_Number_is_zero", number::is_zero;
    "__Number_is_positive__" => "prim_Number_is_positive", number::is_positive;
    "__Number_is_negative__" => "prim_Number_is_negative", number::is_negative;
    "__Number_add__"       => "prim_Number_add", number::add;
    "__Number_subtract__"  => "prim_Number_subtract", number::subtract;
    "__Number_multiply__"  => "prim_Number_multiply", number::multiply;
    "__Number_divide__"    => "prim_Number_divide", number::divide;
    "__Number_sqrt__"      => "prim_Number_sqrt", number::sqrt;
    "__Number_floor__"     => "prim_Number_floor", number::floor;
    "__CChain_get__"       => "prim_CChain_get", cchain::get;
    "__CChain_set__"       => "prim_CChain_set", cchain::set;
    "__CChain_push_CFrame__" => "prim_CChain_push_CFrame", cchain::push_cframe;
    "__CChain_pop_CFrame__" => "prim_CChain_pop_CFrame", cchain::pop_cframe;
    "__Cont_get__"         => "prim_Cont_get", cchain::cont_get;
    "__if_then_else__"     => "prim_if_then_else", control::if_then_else;
    "__if__"               => "prim_if", control::if_;
    "__unless__"           => "prim_unless", control::unless;
    "__CFrame_get_Closure__" => "prim_CFrame_get_Closure", cchain::cframe_get_closure;
    "__CFrame_get_SInfo__" => "prim_CFrame_get_SInfo", cchain::cframe_get_sinfo;
    "__CFrame_new__"       => "prim_CFrame_new", cchain::cframe_new;
    "__Ref_new__"          => "prim_Ref_new", refs::ref_new;
    "__Ref_get__"          => "prim_Ref_get", refs::ref_get;
    "__Ref_set__"          => "prim_Ref_set", refs::ref_set;
    "__PDict_put__"        => "prim_PDict_put", refs::pdict_put;
    "__PDict_lookup_or__"  => "prim_PDict_lookup_or", refs::pdict_lookup_or;
    "__MSeq_new__"         => "prim_MSeq_new", collections::mseq_new;
    "__MSeq_is_empty__"    => "prim_MSeq_is_empty", collections::mseq_is_empty;
    "__MSeq_uncons__"      => "prim_MSeq_uncons", collections::mseq_uncons;
    "__MSeq_set_head__"    => "prim_MSeq_set_head", collections::mseq_set_head;
    "__MSeq_append__"      => "prim_MSeq_append", collections::mseq_append;
    "__Dict_new__"         => "prim_Dict_new", collections::dict_new;
    "__Dict_put__"         => "prim_Dict_put", collections::dict_put;
    "__Dict_lookup_or__"   => "prim_Dict_lookup_or", collections::dict_lookup_or;
    "__Dict_remove__"      => "prim_Dict_remove", collections::dict_remove;
    "__Dict_keys__"        => "prim_Dict_keys", collections::dict_keys;
    "__Dict_copy__"        => "prim_Dict_copy", collections::dict_copy;
    "__LList_append__"     => "prim_LList_append", llist::append;
    "__LList_length__"     => "prim_LList_length", llist::length;
    "__String_find_any__"  => "prim_String_find_any", string::find_any;
    "__String_strip_suffix__" => "prim_String_strip_suffix", string::strip_suffix;
    "__String_length__"    => "prim_String_length", string::length;
    "__String_substring__" => "prim_String_substring", string::substring;
    "__String_find__"      => "prim_String_find", string::find;
    "__Sexp_read__"        => "prim_Sexp_read", string::sexp_read;
    "__String_concat__"    => "prim_String_concat", string::concat;
    "__String_toCode__"    => "prim_String_toCode", string::to_code;
    "__String_fromCode__"  => "prim_String_fromCode", string::from_code;
    "__String_equal__"     => "prim_String_equal", string::equal;
    "__VProc_current__"    => "prim_VProc_current", vprocs::current_vproc;
    "__VProc_suspend__"    => "prim_VProc_suspend", vprocs::suspend;
    "__VProc_resume__"     => "prim_VProc_resume", vprocs::resume;
    "__VProc_wait__"       => "prim_VProc_wait", vprocs::wait;
    "__VProc_id__"         => "prim_VProc_id", vprocs::id;
    "__VProc_kill__"       => "prim_VProc_kill", vprocs::kill;
    "__VProc_spawn__"      => "prim_VProc_spawn", vprocs::spawn;
    "__VProc_start__"      => "prim_VProc_start", vprocs::start;
    "__LazyList_claim__"   => "prim_LazyList_claim", lazy::claim;
    "__LazyList_hold__"    => "prim_LazyList_hold", lazy::hold;
    "__LazyList_publish__" => "prim_LazyList_publish", lazy::publish;
    "__Ref_wait_change__"  => "prim_Ref_wait_change", lazy::ref_wait_change;
    "__OS_get_stdin__"     => "prim_OS_get_stdin", stream::get_stdin;
    "__OS_get_stdout__"    => "prim_OS_get_stdout", stream::get_stdout;
    "__OS_get_stderr__"    => "prim_OS_get_stderr", stream::get_stderr;
    "__OS_read__"          => "prim_OS_read", stream::read;
    "__OS_write__"         => "prim_OS_write", stream::write;
    "__OS_close__"         => "prim_OS_close", stream::close;
    "__OS_dup__"           => "prim_OS_dup", stream::dup;
    "__OS_pipe__"          => "prim_OS_pipe", stream::pipe;
    "__OS_wait_readable__" => "prim_OS_wait_readable", stream::wait_readable;
    "__OS_wait_writable__" => "prim_OS_wait_writable", stream::wait_writable;
    "__OS_get_String__"    => "prim_OS_get_String", stream::get_string;
    "__OS_open__"          => "prim_OS_open", stream::open;
    "__OS_open_tracked__"  => "prim_OS_open_tracked", stream::open_tracked;
    "__OS_chdir__"         => "prim_OS_chdir", stream::chdir;
    "__OS_getcwd__"        => "prim_OS_getcwd", stream::getcwd;
    "__OS_realpath__"      => "prim_OS_realpath", stream::realpath;
    "__OS_listdir__"       => "prim_OS_listdir", stream::listdir;
    "__OS_get_envDict__"   => "prim_OS_get_envDict", stream::get_env_dict;
    "__OS_getpid__"        => "prim_OS_getpid", stream::getpid;
    "__OS_sysconf__"       => "prim_OS_sysconf", stream::sysconf;
    "__OS_spawn__"         => "prim_OS_spawn", process::spawn;
    "__OS_spawn_detached__" => "prim_OS_spawn_detached", process::spawn_detached;
    "__OS_poll__"          => "prim_OS_poll", process::poll;
    "__OS_wait__"          => "prim_OS_wait", process::wait;
    "__OS_kill__"          => "prim_OS_kill", process::kill;
    "__OS_trap_interrupt__" => "prim_OS_trap_interrupt", process::trap_interrupt;
}

pub fn lookup(name: &str) -> Option<&'static PrimDef> {
    PRIMS.iter().find(|p| p.name == name)
}

pub fn lookup_pyname(pyname: &str) -> Option<&'static PrimDef> {
    PRIMS.iter().find(|p| p.pyname == pyname)
}

/// Take the first n operands, or fail with "<name>: missing argument(s)".
pub fn take_n(vp: &mut VProc, name: &str, n: usize) -> Result<Vec<crate::value::Value>, VmError> {
    if vp.olist.len() < n {
        return err(format!("{}: missing argument(s)", name));
    }
    Ok(vp.olist.drain(..n).collect())
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
