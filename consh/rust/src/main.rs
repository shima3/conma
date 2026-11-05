//! ConSh run directly in Rust. The ConMa version (consh/include) is the
//! reference: for the same input, the output and the exit status are the
//! same.
//!
//!   consh-rs run <syntax tree> <script> [<arg> ...]
//!
//! The syntax tree is what `consh_parse --pos` writes for the script.

mod ast;
mod chan;
mod eval;
mod expand;
mod glob;
mod os;
mod value;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 || args[1] != "run" {
        os::print_error("usage: consh-rs run <syntax tree> <script> [<arg> ...]\n");
        std::process::exit(2);
    }
    let text = match std::fs::read_to_string(&args[2]) {
        Ok(t) => t,
        Err(e) => {
            os::print_error(&format!("consh: {}: {}\n", args[2], os::strerror(&e)));
            std::process::exit(1);
        }
    };
    let mut lines = Vec::new();
    for (i, line) in text.lines().enumerate() {
        match ast::read_line(line) {
            Ok(v) => lines.push(v),
            Err(msg) => {
                os::print_error(&format!("consh: {}:{}: {}\n", args[2], i + 1, msg));
                std::process::exit(1);
            }
        }
    }
    // Run in a thread with a large stack, as the threads of the shell do.
    let (script, rest) = (args[3].clone(), args[4..].to_vec());
    let code = eval::spawn(move || eval::eval_run(&script, rest, &lines)).join().unwrap_or(1);
    std::process::exit(code);
}
