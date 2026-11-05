//! The syntax tree that `consh_parse --pos` writes (consh/AST.md): one
//! S-expression per command line. A list whose first element is a symbol is
//! a node; with --pos, its second element is the location `(line col)`.

use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Sexp {
    List(Arc<Vec<Sexp>>),
    Str(Arc<str>),
    Sym(Arc<str>),
    Num(i64),
}

/// A location (line, column), both from 1.
pub type Pos = Option<(i64, i64)>;

impl Sexp {
    pub fn list(&self) -> &[Sexp] {
        match self {
            Sexp::List(l) => l,
            _ => &[],
        }
    }
    pub fn str(&self) -> &str {
        match self {
            Sexp::Str(s) | Sexp::Sym(s) => s,
            _ => "",
        }
    }
    pub fn is_list(&self) -> bool {
        matches!(self, Sexp::List(_))
    }
    /// The node type (the first symbol of a list), or "".
    pub fn tag(&self) -> &str {
        match self.list().first() {
            Some(Sexp::Sym(s)) => s,
            _ => "",
        }
    }
    /// The location of a node.
    pub fn pos(&self) -> Pos {
        match self.list().get(1) {
            Some(Sexp::List(l)) if l.len() == 2 => match (&l[0], &l[1]) {
                (Sexp::Num(a), Sexp::Num(b)) => Some((*a, *b)),
                _ => None,
            },
            _ => None,
        }
    }
    /// The children of a node (after its type and location).
    pub fn children(&self) -> &[Sexp] {
        let l = self.list();
        if l.is_empty() {
            return l;
        }
        if self.pos().is_some() {
            &l[2..]
        } else {
            &l[1..]
        }
    }
    /// An identity for a node, for tables keyed by node.
    pub fn id(&self) -> usize {
        match self {
            Sexp::List(l) => Arc::as_ptr(l) as usize,
            _ => 0,
        }
    }
}

pub fn read_line(text: &str) -> Result<Sexp, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let v = read(&chars, &mut i)?;
    skip(&chars, &mut i);
    if i != chars.len() {
        return Err(format!("unexpected characters after an S-expression at column {}", i + 1));
    }
    Ok(v)
}

fn skip(c: &[char], i: &mut usize) {
    while *i < c.len() && matches!(c[*i], ' ' | '\t' | '\n' | '\r') {
        *i += 1;
    }
}

fn read(c: &[char], i: &mut usize) -> Result<Sexp, String> {
    skip(c, i);
    if *i >= c.len() {
        return Err(format!("expected a value at column {}", *i + 1));
    }
    match c[*i] {
        '(' => {
            *i += 1;
            let mut items = Vec::new();
            loop {
                skip(c, i);
                if *i >= c.len() {
                    return Err(format!("unterminated list at column {}", *i + 1));
                }
                if c[*i] == ')' {
                    *i += 1;
                    return Ok(Sexp::List(Arc::new(items)));
                }
                items.push(read(c, i)?);
            }
        }
        '"' => {
            *i += 1;
            let mut s = String::new();
            loop {
                if *i >= c.len() {
                    return Err(format!("unterminated string at column {}", *i + 1));
                }
                let ch = c[*i];
                *i += 1;
                match ch {
                    '"' => return Ok(Sexp::Str(Arc::from(s))),
                    '\\' if *i < c.len() => {
                        let e = c[*i];
                        *i += 1;
                        s.push(match e {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            other => other,
                        });
                    }
                    other => s.push(other),
                }
            }
        }
        _ => {
            let start = *i;
            while *i < c.len() && !matches!(c[*i], ' ' | '\t' | '\n' | '\r' | '(' | ')' | '"') {
                *i += 1;
            }
            let word: String = c[start..*i].iter().collect();
            if word.starts_with(|ch: char| ch.is_ascii_digit()) {
                word.parse::<i64>().map(Sexp::Num).map_err(|_| format!("invalid number at column {}", start + 1))
            } else {
                Ok(Sexp::Sym(Arc::from(word)))
            }
        }
    }
}
