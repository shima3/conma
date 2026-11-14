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

/// Whether the character at byte i of text (the start of a character) is white space, and its
/// length in bytes. ASCII is looked at as a byte; the rest is decoded.
fn space_at(text: &str, i: usize) -> (bool, usize) {
    let c = text.as_bytes()[i];
    if c < 0x80 {
        return ((c as char).is_whitespace(), 1);
    }
    let ch = text[i..].chars().next().unwrap();
    (ch.is_whitespace(), ch.len_utf8())
}

/// The next token of text from byte *pos, as the regular expression
/// `"(?:[^"\\]|\\.)*"|\(|\)|[^\s()]+` finds them, or None at the end. It looks at the bytes,
/// which is enough, as the delimiters are ASCII and the bytes of a character that is not ASCII
/// are all 0x80 or more (decoding only to test white space), instead of making the vector of
/// every character of a module.
fn next_token<'a>(text: &'a str, pos: &mut usize) -> Option<&'a str> {
    let b = text.as_bytes();
    let mut i = *pos;
    while i < b.len() {
        let c = b[i];
        if c == b'(' || c == b')' {
            *pos = i + 1;
            return Some(&text[i..i + 1]);
        }
        let (space, len) = space_at(text, i);
        if space {
            i += len;
            continue;
        }
        if c == b'"' {
            // A quoted string: any character but " and \, or \ and any character but a newline.
            let mut j = i + 1;
            while j < b.len() {
                let d = b[j];
                if d == b'"' {
                    *pos = j + 1;
                    return Some(&text[i..j + 1]);
                }
                if d == b'\\' {
                    if j + 1 < b.len() && b[j + 1] != b'\n' {
                        j += 2;
                        continue;
                    }
                    break;
                }
                j += 1;
            }
            // not closed: a bare token
        }
        // A bare token: up to whitespace or a parenthesis.
        let mut j = i;
        while j < b.len() {
            let d = b[j];
            if d == b'(' || d == b')' {
                break;
            }
            let (space, len) = space_at(text, j);
            if space {
                break;
            }
            j += len;
        }
        *pos = j;
        return Some(&text[i..j]);
    }
    *pos = i;
    None
}

/// Read one S-expression from the front of text. Without recursion, so that
/// the deep nesting of a module does not overflow the stack.
pub fn parse(text: &str) -> Result<Sexp, String> {
    let mut stack: Vec<Vec<Sexp>> = Vec::new();
    let mut pos = 0;
    while let Some(t) = next_token(text, &mut pos) {
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
