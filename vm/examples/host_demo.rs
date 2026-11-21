//! An example of a program that embeds the ConMa VM and adds functions of the host (docs/HOST-ja.md,
//! docs/dev/PLATFORM-PLAN-ja.md 4.9, stage P7). It is the conma command with these functions and the
//! platform feature "demo":
//!
//! * demo.add a b ...: the sum of the Numbers (returns at once)
//! * demo.info name tags: a Dict {name, count, short}: the name, the number of tags and whether the
//!   name is shorter than 4 characters (returns at once; Dicts and Booleans go back)
//! * demo.delay ms text: text, after ms milliseconds (waits: a thread finishes the call)
//! * demo.drop: drops its Done without finishing it (the call ends with an error)
//!
//! Build and run it (test/host-api.sh does):
//!
//!     cargo build --release --example host_demo
//!     CONMA_RUNNER=vm/target/release/examples/host_demo bin/lang conjs conjs/examples/host_demo.con.js

use conma::{Host, HostValue};

// The allocator of the VM, much faster than that of the system for the small blocks of the VM.
#[global_allocator]
static GLOBAL: conma::Pool = conma::Pool::new();

fn main() {
    let mut host = Host::new();
    host.function("demo.add", |args| {
        let mut sum = 0.0;
        for a in args {
            match a {
                HostValue::Num(x) => sum += x,
                _ => return Err("demo.add: expected Numbers".to_string()),
            }
        }
        Ok(HostValue::Num(sum))
    });
    host.function("demo.info", |args| match args {
        [HostValue::Str(name), HostValue::List(tags)] => Ok(HostValue::Dict(vec![
            ("name".to_string(), HostValue::Str(name.clone())),
            ("count".to_string(), HostValue::Num(tags.len() as f64)),
            ("short".to_string(), HostValue::Bool(name.chars().count() < 4)),
        ])),
        _ => Err("demo.info: expected a name and a list of tags".to_string()),
    });
    host.waiting("demo.delay", |args, done| {
        let (ms, text) = match args {
            [HostValue::Num(ms), HostValue::Str(text)] if *ms >= 0.0 => (*ms, text.clone()),
            _ => return done.finish(Err("demo.delay: expected milliseconds and a text".to_string())),
        };
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs_f64(ms / 1000.0));
            done.finish(Ok(HostValue::Str(text)));
        });
    });
    host.waiting("demo.drop", |_args, done| drop(done));
    host.feature("demo");
    std::process::exit(conma::run_main_with(std::env::args().skip(1), host));
}
