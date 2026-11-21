//! conma — run a ConMa VProc to completion, as vm/python/vproc_run.py does. The VM is the
//! library of this crate (src/lib.rs, which says how to use the command).

#[global_allocator]
static GLOBAL: conma::Pool = conma::Pool::new();

fn main() {
    std::process::exit(conma::run_main(std::env::args().skip(1)));
}
