//! The OS functions of the native VM, one file for each system: unix.rs for
//! macOS and Linux, windows.rs for Windows (docs/dev/PLATFORM-PLAN-ja.md
//! 4.11). The VM for wasm32 has its own (vm/wasm/src/sys.rs).

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::*;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;
