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
    let mut pos = 0;
    parse_at(text, &mut pos)
}

/// Read one S-expression of text from byte *pos, leaving *pos after it.
pub fn parse_at(text: &str, pos: &mut usize) -> Result<Sexp, String> {
    let mut stack: Vec<Vec<Sexp>> = Vec::new();
    while let Some(t) = next_token(text, pos) {
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

/// What a byte is to skip_at: OTHER, an ASCII byte of a bare token; SPACE, ASCII white space;
/// OPEN and CLOSE, the parentheses; QUOTE; and HIGH, a byte of a character that is not ASCII
/// (decoded by space_at to tell white space).
const OTHER: u8 = 0;
const SPACE: u8 = 1;
const OPEN: u8 = 2;
const CLOSE: u8 = 3;
const QUOTE: u8 = 4;
const HIGH: u8 = 5;

const BYTE_CLASS: [u8; 256] = {
    let mut t = [OTHER; 256];
    let mut i = 0;
    while i < 256 {
        t[i] = if i >= 0x80 {
            HIGH
        } else if (i as u8 as char).is_whitespace() {
            SPACE
        } else if i == b'(' as usize {
            OPEN
        } else if i == b')' as usize {
            CLOSE
        } else if i == b'"' as usize {
            QUOTE
        } else {
            OTHER
        };
        i += 1;
    }
    t
};

/// Skip one S-expression of text from byte *pos, as parse_at reads it, without making it: the
/// tokens of next_token, looked at byte by byte without taking them, the ASCII bytes by a table.
pub fn skip_at(text: &str, pos: &mut usize) -> Result<(), String> {
    let b = text.as_bytes();
    let mut i = *pos;
    let mut depth = 0usize;
    while i < b.len() {
        match BYTE_CLASS[b[i] as usize] {
            OPEN => {
                depth += 1;
                i += 1;
                continue;
            }
            CLOSE => {
                if depth == 0 {
                    return Err("unexpected )".into());
                }
                depth -= 1;
                i += 1;
                if depth == 0 {
                    *pos = i;
                    return Ok(());
                }
                continue;
            }
            SPACE => {
                i += 1;
                continue;
            }
            HIGH => {
                let (space, len) = space_at(text, i);
                if space {
                    i += len;
                    continue;
                }
            }
            _ => {}
        }
        let mut closed = false;
        if b[i] == b'"' {
            // A quoted string, as next_token reads it
            let mut j = i + 1;
            while let Some(k) = b[j..].iter().position(|&d| d == b'"' || d == b'\\') {
                j += k;
                if b[j] == b'"' {
                    i = j + 1;
                    closed = true;
                    break;
                }
                if j + 1 < b.len() && b[j + 1] != b'\n' {
                    j += 2;
                    continue;
                }
                break;
            }
        }
        if !closed {
            // A bare token: up to white space or a parenthesis
            while i < b.len() {
                match BYTE_CLASS[b[i] as usize] {
                    OTHER | QUOTE => i += 1,
                    HIGH => {
                        let (space, len) = space_at(text, i);
                        if space {
                            break;
                        }
                        i += len;
                    }
                    _ => break,
                }
            }
        }
        if depth == 0 {
            *pos = i;
            return Ok(());
        }
    }
    Err("EOF".into())
}

/// The next token of text from byte pos and the byte where it starts, without consuming it.
pub fn peek_at(text: &str, pos: usize) -> Option<(&str, usize)> {
    let mut p = pos;
    let t = next_token(text, &mut p)?;
    Some((t, t.as_ptr() as usize - text.as_ptr() as usize))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// skip_at stops where parse_at does, on texts with strings, escapes, quotes inside bare
    /// tokens, strings that are not closed, and white space that is not ASCII.
    #[test]
    fn skip_at_stops_where_parse_at_does() {
        let texts = [
            "(a b c) rest",
            "((a (b)) \"x y\" (c)) rest",
            "(G 1 \"name\" (Function (1 2) (Body (SInfo (1 2) \"/a b/c.se\" \"1\" \"2\")))) rest",
            "(a \"esc \\\" ( ) \\\\\" b) rest",
            "(a\"b c\"d e) rest",
            "(a \"not closed\\\n x) rest",
            "(a\u{3000}b\u{00a0}(c)\u{2028}) rest",
            "(\u{3042}\u{3044} \"\u{3046} (\" x) rest",
            "(a \"x\" \"\" ()) rest",
            "atom rest",
            "\"str\" rest",
        ];
        for t in texts {
            let mut p = 0;
            parse_at(t, &mut p).unwrap();
            let mut q = 0;
            skip_at(t, &mut q).unwrap();
            assert_eq!(p, q, "{:?}", t);
        }
        for t in ["(a (b)", "(a \"(\"", ")"] {
            let mut p = 0;
            let mut q = 0;
            assert!(parse_at(t, &mut p).is_err() && skip_at(t, &mut q).is_err(), "{:?}", t);
        }
    }
}
