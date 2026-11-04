//! S-expressions: reading a module and writing traces, as `parse_sexp` and
//! `sexp_to_str` in test/vproc_step.py do.

use std::rc::Rc;

/// An atom keeps its text as written (a String keeps its quotes and escapes).
#[derive(Clone, Debug)]
pub enum Sexp {
    Atom(Rc<str>),
    List(Rc<Vec<Sexp>>),
}

impl Sexp {
    pub fn atom(s: &str) -> Sexp {
        Sexp::Atom(Rc::from(s))
    }
    pub fn list(items: Vec<Sexp>) -> Sexp {
        Sexp::List(Rc::new(items))
    }
    pub fn as_atom(&self) -> Option<&str> {
        match self {
            Sexp::Atom(a) => Some(a),
            Sexp::List(_) => None,
        }
    }
    pub fn as_list(&self) -> Option<&[Sexp]> {
        match self {
            Sexp::List(l) => Some(l),
            Sexp::Atom(_) => None,
        }
    }
    /// The first element of a non-empty list, if it is an atom.
    pub fn tag(&self) -> Option<&str> {
        self.as_list().and_then(|l| l.first()).and_then(|x| x.as_atom())
    }
    /// The position `(0 0)` that the VM writes for nodes it makes.
    pub fn pos0() -> Sexp {
        Sexp::list(vec![Sexp::atom("0"), Sexp::atom("0")])
    }
}

/// Tokens as the regular expression `"(?:[^"\\]|\\.)*"|\(|\)|[^\s()]+` finds them.
fn tokenize(text: &str) -> Vec<&str> {
    let b: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let end_of = |k: usize| if k < b.len() { b[k].0 } else { text.len() };
    while i < b.len() {
        let c = b[i].1;
        if c == '(' || c == ')' {
            out.push(&text[b[i].0..end_of(i + 1)]);
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '"' {
            // A quoted string: any character but " and \, or \ and any character but a newline.
            let mut j = i + 1;
            let mut closed = false;
            while j < b.len() {
                let d = b[j].1;
                if d == '"' {
                    closed = true;
                    break;
                }
                if d == '\\' {
                    if j + 1 < b.len() && b[j + 1].1 != '\n' {
                        j += 2;
                        continue;
                    }
                    break;
                }
                j += 1;
            }
            if closed {
                out.push(&text[b[i].0..end_of(j + 1)]);
                i = j + 1;
                continue;
            }
        }
        // A bare token: up to whitespace or a parenthesis.
        let mut j = i;
        while j < b.len() && !(b[j].1.is_whitespace() || b[j].1 == '(' || b[j].1 == ')') {
            j += 1;
        }
        out.push(&text[b[i].0..end_of(j)]);
        i = j;
    }
    out
}

/// Read one S-expression from the front of text. Without recursion, so that
/// the deep nesting of a module does not overflow the stack.
pub fn parse(text: &str) -> Result<Sexp, String> {
    let toks = tokenize(text);
    let mut stack: Vec<Vec<Sexp>> = Vec::new();
    for t in toks {
        match t {
            "(" => stack.push(Vec::new()),
            ")" => {
                let done = match stack.pop() {
                    Some(l) => Sexp::list(l),
                    None => return Err("unexpected )".into()),
                };
                match stack.last_mut() {
                    Some(parent) => parent.push(done),
                    None => return Ok(done),
                }
            }
            a => {
                let atom = Sexp::atom(a);
                match stack.last_mut() {
                    Some(parent) => parent.push(atom),
                    None => return Ok(atom),
                }
            }
        }
    }
    Err("EOF".into())
}

/// Length of x on one line, stopping early once it exceeds limit.
fn inline_len(x: &Sexp, limit: isize) -> isize {
    match x {
        Sexp::Atom(a) => a.chars().count() as isize,
        Sexp::List(l) => {
            if l.is_empty() {
                return 2;
            }
            let mut n: isize = 1;
            for i in l.iter() {
                n += inline_len(i, limit - n) + 1;
                if n > limit {
                    return n;
                }
            }
            n
        }
    }
}

fn write(x: &Sexp, indent: usize, out: &mut String) {
    match x {
        Sexp::Atom(a) => out.push_str(a),
        Sexp::List(l) => {
            if l.is_empty() {
                out.push_str("()");
                return;
            }
            if inline_len(x, 80) <= 80 {
                out.push('(');
                for (k, i) in l.iter().enumerate() {
                    if k > 0 {
                        out.push(' ');
                    }
                    write(i, 0, out);
                }
                out.push(')');
                return;
            }
            out.push('(');
            for (k, i) in l.iter().enumerate() {
                if k > 0 {
                    out.push('\n');
                    for _ in 0..indent + 1 {
                        out.push_str("  ");
                    }
                }
                write(i, indent + 1, out);
            }
            out.push(')');
        }
    }
}

/// The text of x, on one line when it fits in 80 characters, otherwise with
/// each element on its own line, indented two spaces per level.
pub fn to_string(x: &Sexp) -> String {
    let mut out = String::new();
    write(x, 0, &mut out);
    out
}
