//! ConSh run directly in Rust. The ConMa version (consh/include) is the
//! reference: for the same input, the output and the exit status are the
//! same.
//!
//!   consh-rs run <script> [<arg> ...]            run a script
//!   consh-rs dump <script>                       write its syntax tree as consh_parse --pos does
//!   consh-rs interactive [<arg> ...]             interactive mode (the standard input)
//!   consh-rs run-ast <syntax tree> <script> [<arg> ...]
//!                                                run the syntax tree that consh_parse --pos wrote
//!                                                for the script

mod ast;
mod chan;
mod eval;
mod expand;
mod glob;
mod os;
mod parse;
mod value;

fn usage() -> ! {
    os::print_error(
        "usage: consh-rs run <script> [<arg> ...]\n       consh-rs dump <script>\n       consh-rs interactive [<arg> ...]\n       consh-rs run-ast <syntax tree> <script> [<arg> ...]\n",
    );
    std::process::exit(2);
}

/// The command lines of a script, or the error message and status 1.
fn parse_file(path: &str) -> Vec<ast::Sexp> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            os::print_error(&format!("consh: __OS_open__: {}: {}\n", os::strerror(&e), path));
            std::process::exit(1);
        }
    };
    match parse::parse(&text, 1) {
        Ok(lines) => lines,
        Err((full, _)) => {
            os::print_error(&format!("consh: {}:{}\n", path, full));
            std::process::exit(1);
        }
    }
}

fn read_ast(path: &str) -> Vec<ast::Sexp> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            os::print_error(&format!("consh: {}: {}\n", path, os::strerror(&e)));
            std::process::exit(1);
        }
    };
    let mut lines = Vec::new();
    for (i, line) in text.lines().enumerate() {
        match ast::read_line(line) {
            Ok(v) => lines.push(v),
            Err(msg) => {
                os::print_error(&format!("consh: {}:{}: {}\n", path, i + 1, msg));
                std::process::exit(1);
            }
        }
    }
    lines
}

/// Run in a thread with a large stack, as the threads of the shell do.
fn run(script: String, args: Vec<String>, lines: Vec<ast::Sexp>) -> ! {
    let code = eval::spawn(move || eval::eval_run(&script, args, &lines)).join().unwrap_or(1);
    std::process::exit(code);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("run") if args.len() >= 3 => {
            let lines = parse_file(&args[2]);
            run(args[2].clone(), args[3..].to_vec(), lines)
        }
        Some("dump") if args.len() == 3 => {
            let mut out = String::new();
            for line in parse_file(&args[2]) {
                parse::write(&line, &mut out);
                out.push('\n');
            }
            print!("{}", out);
        }
        Some("interactive") => {
            let rest = args[2..].to_vec();
            let code = eval::spawn(move || eval::repl_run(rest)).join().unwrap_or(1);
            std::process::exit(code);
        }
        Some("run-ast") if args.len() >= 4 => {
            let lines = read_ast(&args[2]);
            run(args[3].clone(), args[4..].to_vec(), lines)
        }
        _ => usage(),
    }
}
