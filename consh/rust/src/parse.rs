//! The parser of ConSh: a port of the Parser of bin/consh_parse (and of
//! consh_parser.se, which follows the same steps). It reads a script by
//! recursive descent over its characters and makes the syntax tree that
//! `consh_parse --pos` writes (consh/AST.md), one node per command line.
//!
//! A line continuation (\ and a newline) is skipped before a character is
//! read, except in '...', $'...' and comments. sep_end is the position just
//! after the start of a line, a space, ;, | or &: a # there starts a comment.

use crate::ast::Sexp;
use std::sync::Arc;

/// A syntax error: the message and the position (an index into the characters).
pub struct ParseError {
    pub msg: String,
    pub pos: usize,
}

type P<T> = Result<T, ParseError>;

const END: char = '\0';
const SPECIALS: &[char] = &[';', '\'', '(', ')', '{', '}', '|', '&', '>', '<', '"', '$', '\\'];
const TERMINATORS: &[&str] = &["then", "elif", "else", "fi", "do", "done"];
const KEYWORDS: &[&str] = &[
    "function", "export", "return", "if", "for", "break", "continue", "then", "elif", "else", "fi", "do", "done",
];

fn is_space(c: char) -> bool {
    c == ' ' || c == '\t'
}
fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic()
}
fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}
fn is_ident_char(c: char) -> bool {
    is_ident_start(c) || is_digit(c)
}

/// repr() of Python for a one-character string.
fn repr_char(c: char) -> String {
    let s = c.to_string();
    repr_str(&s)
}

pub fn repr_str(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
    let mut out = String::new();
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// Where the nodes are: the start of each line, and the number of the first line.
struct Lines {
    starts: Vec<usize>,
    first: i64,
}

impl Lines {
    fn new(text: &[char], first: i64) -> Lines {
        let mut starts = vec![0];
        for (i, c) in text.iter().enumerate() {
            if *c == '\n' {
                starts.push(i + 1);
            }
        }
        Lines { starts, first }
    }
    /// (line, column) of an index, both from 1.
    fn at(&self, pos: usize) -> (i64, i64) {
        // The line is the number of newlines before pos.
        let k = match self.starts.binary_search(&pos) {
            Ok(k) => k,
            Err(k) => k - 1,
        };
        (self.first + k as i64, (pos - self.starts[k]) as i64 + 1)
    }
}

struct Parser<'a> {
    text: &'a [char],
    pos: usize,
    sep_end: usize,
    lines: &'a Lines,
}

fn s(x: &str) -> Sexp {
    Sexp::Str(Arc::from(x))
}

/// The location of a node (the second element of a list).
fn pos_of(n: &Sexp) -> Sexp {
    n.list()[1].clone()
}

impl<'a> Parser<'a> {
    fn err<T>(&self, msg: String) -> P<T> {
        Err(ParseError { msg, pos: self.pos })
    }

    fn loc(&self, pos: usize) -> Sexp {
        let (l, c) = self.lines.at(pos);
        Sexp::List(Arc::new(vec![Sexp::Num(l), Sexp::Num(c)]))
    }

    fn node(&self, kind: &str, loc: Sexp, parts: Vec<Sexp>) -> Sexp {
        let mut v = Vec::with_capacity(parts.len() + 2);
        v.push(Sexp::Sym(Arc::from(kind)));
        v.push(loc);
        v.extend(parts);
        Sexp::List(Arc::new(v))
    }

    // ── Reading characters ──

    fn raw(&self, i: usize) -> char {
        if i < self.text.len() {
            self.text[i]
        } else {
            END
        }
    }

    fn cont_len(&self, i: usize) -> usize {
        if self.raw(i) == '\\' {
            if self.raw(i + 1) == '\n' {
                return 2;
            }
            if self.raw(i + 1) == '\r' && self.raw(i + 2) == '\n' {
                return 3;
            }
        }
        0
    }

    fn skip(&mut self) {
        loop {
            let n = self.cont_len(self.pos);
            if n == 0 {
                return;
            }
            self.pos += n;
        }
    }

    /// The index k characters ahead, without line continuations.
    fn index(&self, mut k: usize) -> usize {
        let mut i = self.pos;
        loop {
            loop {
                let n = self.cont_len(i);
                if n == 0 {
                    break;
                }
                i += n;
            }
            if k == 0 || i >= self.text.len() {
                return i;
            }
            i += 1;
            k -= 1;
        }
    }

    fn peek(&self) -> char {
        self.peek_at(0)
    }

    fn peek_at(&self, k: usize) -> char {
        self.raw(self.index(k))
    }

    fn peek_str(&self, n: usize) -> String {
        (0..n).map(|k| self.peek_at(k)).filter(|c| *c != END).collect()
    }

    fn advance(&mut self, n: usize) {
        for _ in 0..n {
            self.skip();
            self.pos += 1;
        }
    }

    fn at_newline_at(&self, k: usize) -> bool {
        let c = self.peek_at(k);
        c == '\n' || (c == '\r' && self.peek_at(k + 1) == '\n')
    }

    fn at_newline(&self) -> bool {
        self.at_newline_at(0)
    }

    fn consume_newline(&mut self) {
        let n = if self.peek() == '\r' { 2 } else { 1 };
        self.advance(n);
        self.mark_sep();
    }

    fn mark_sep(&mut self) {
        self.skip();
        self.sep_end = self.pos;
    }

    fn skip_spaces(&mut self) -> usize {
        let mut n = 0;
        while is_space(self.peek()) {
            self.advance(1);
            n += 1;
        }
        if n > 0 {
            self.mark_sep();
        }
        n
    }

    fn at_comment(&mut self) -> bool {
        self.skip();
        self.peek() == '#' && self.pos == self.sep_end
    }

    fn skip_comment(&mut self) {
        // Line continuations stay in a comment.
        while self.pos < self.text.len() && self.text[self.pos] != '\n' && !(self.text[self.pos] == '\r' && self.raw(self.pos + 1) == '\n') {
            self.pos += 1;
        }
    }

    fn is_word_start(&self) -> bool {
        let c = self.peek();
        !(c == END || is_space(c) || self.at_newline() || (SPECIALS.contains(&c) && !matches!(c, '"' | '\'' | '$' | '\\')))
    }

    fn is_bare(&self) -> bool {
        self.is_word_start() && !matches!(self.peek(), '"' | '\'' | '$' | '\\')
    }

    fn at_stmt_end(&mut self, term: Option<char>) -> bool {
        let c = self.peek();
        c == END || self.at_newline() || c == ';' || Some(c) == term || self.at_comment()
    }

    fn describe(&self) -> String {
        let c = self.peek();
        if c == END {
            return "end of input".into();
        }
        if let Some(kw) = self.keyword() {
            if TERMINATORS.contains(&kw) {
                return kw.into();
            }
        }
        if self.at_newline() {
            return "newline".into();
        }
        repr_char(c)
    }

    fn expect(&mut self, c: char) -> P<()> {
        if self.peek() != c {
            return self.err(format!("expected {}, but found {}", repr_char(c), self.describe()));
        }
        self.advance(1);
        Ok(())
    }

    // ── Scripts and command lines ──

    fn program(&mut self) -> P<Vec<Sexp>> {
        self.mark_sep();
        let script = self.script(None)?;
        if self.peek() != END {
            return self.err(format!("unexpected {}", self.describe()));
        }
        Ok(script.1)
    }

    /// The command lines of a script, and the location where it starts.
    fn script(&mut self, term: Option<char>) -> P<(Sexp, Vec<Sexp>)> {
        let loc = self.loc(self.index(0));
        let mut lines = Vec::new();
        loop {
            let stmts = self.command_line(term)?;
            if !stmts.is_empty() {
                let l = pos_of(&stmts[0]);
                lines.push(self.node("line", l, stmts));
            }
            if !self.at_newline() {
                break;
            }
            self.consume_newline();
        }
        if let Some(t) = term {
            if self.peek() != t {
                return self.err(format!("expected {}, but found {}", repr_char(t), self.describe()));
            }
        }
        Ok((loc, lines))
    }

    fn script_node(&mut self, term: Option<char>) -> P<Sexp> {
        let (loc, lines) = self.script(term)?;
        Ok(self.node("script", loc, lines))
    }

    fn at_line_end(&self, term: Option<char>) -> bool {
        let c = self.peek();
        c == END || self.at_newline() || Some(c) == term || self.at_terminator()
    }

    fn command_line(&mut self, term: Option<char>) -> P<Vec<Sexp>> {
        let mut stmts = Vec::new();
        loop {
            self.skip_spaces();
            if self.at_comment() {
                self.skip_comment();
                break;
            }
            if self.at_line_end(term) {
                break;
            }
            stmts.push(self.statement(term)?);
            self.skip_spaces();
            if self.peek() == ';' {
                self.advance(1);
                self.mark_sep();
                continue;
            }
            if self.at_comment() {
                self.skip_comment();
                break;
            }
            if self.at_line_end(term) {
                break;
            }
            return self.err(format!("unexpected {}", self.describe()));
        }
        Ok(stmts)
    }

    // ── Statements ──

    /// A keyword: only before a space, a newline, ;, |, & or the end of input.
    fn keyword(&self) -> Option<&'static str> {
        for kw in KEYWORDS {
            let n = kw.chars().count();
            if self.peek_str(n) == *kw {
                let c = self.peek_at(n);
                if c == END || is_space(c) || matches!(c, ';' | '|' | '&') || self.at_newline_at(n) {
                    return Some(kw);
                }
            }
        }
        None
    }

    fn at_terminator(&self) -> bool {
        matches!(self.keyword(), Some(kw) if TERMINATORS.contains(&kw))
    }

    fn expect_keyword(&mut self, kw: &str) -> P<usize> {
        if self.keyword() != Some(kw) {
            return self.err(format!("expected {}, but found {}", kw, self.describe()));
        }
        let pos = self.index(0);
        self.advance(kw.len());
        Ok(pos)
    }

    fn statement(&mut self, term: Option<char>) -> P<Sexp> {
        let pos = self.index(0);
        let loc = self.loc(pos);
        let kw = self.keyword();
        match kw {
            Some("return") => {
                self.advance(6);
                if self.skip_spaces() == 0 || !self.is_word_start() || self.at_comment() {
                    return self.err("expected a word after return".into());
                }
                let w = self.word()?;
                return Ok(self.node("return", loc, vec![w]));
            }
            Some(k @ ("break" | "continue")) => {
                self.advance(k.len());
                let mut parts = Vec::new();
                if self.skip_spaces() > 0 && !self.at_comment() && !self.at_stmt_end(term) {
                    if !is_ident_start(self.peek()) {
                        return self.err(format!("expected a variable name after {}, but found {}", k, self.describe()));
                    }
                    parts.push(s(&self.ident()));
                }
                return Ok(self.node(k, loc, parts));
            }
            Some("export") => {
                self.advance(6);
                if self.peek() == END || self.at_newline() {
                    return self.err("export followed by a newline".into());
                }
                let mut items = Vec::new();
                while self.skip_spaces() > 0 && is_ident_start(self.peek()) {
                    let iloc = self.loc(self.index(0));
                    let name = self.ident();
                    if self.peek() == '=' {
                        self.advance(1);
                        let v = self.value()?;
                        items.push(self.item(iloc, &name, v));
                    } else {
                        items.push(self.node("name", iloc, vec![s(&name)]));
                    }
                }
                return Ok(self.node("export", loc, items));
            }
            Some("function") => {
                self.advance(8);
                if self.peek() == END || self.at_newline() {
                    return self.err("function followed by a newline".into());
                }
                self.skip_spaces();
                if !is_ident_start(self.peek()) {
                    return self.err(format!("expected a function name, but found {}", self.describe()));
                }
                let name = self.ident();
                self.skip_spaces();
                if self.peek() != '(' {
                    return self.err(format!("expected '(', but found {}", self.describe()));
                }
                let params = self.params()?;
                self.skip_spaces();
                self.expect('{')?;
                let body = self.script_node(Some('}'))?;
                self.advance(1);
                return Ok(self.node("function", loc, vec![s(&name), params, body]));
            }
            _ => {}
        }
        let mut defs = Vec::new();
        while self.at_def() {
            defs.push(self.def()?);
            if self.skip_spaces() == 0 || !self.at_def() {
                break;
            }
        }
        if !defs.is_empty() && self.at_stmt_end(term) {
            let l = pos_of(&defs[0]);
            return Ok(self.node("assign", l, defs));
        }
        self.command_list(Some(defs))
    }

    /// (def pos "name" value), or (def pos "name") for an empty value.
    fn item(&self, loc: Sexp, name: &str, v: Option<Sexp>) -> Sexp {
        let mut parts = vec![s(name)];
        parts.extend(v);
        self.node("def", loc, parts)
    }

    fn at_def(&self) -> bool {
        if !is_ident_start(self.peek()) {
            return false;
        }
        let mut k = 1;
        while is_ident_char(self.peek_at(k)) {
            k += 1;
        }
        self.peek_at(k) == '='
    }

    fn def(&mut self) -> P<Sexp> {
        let loc = self.loc(self.index(0));
        let name = self.ident();
        self.advance(1);
        let v = self.value()?;
        Ok(self.item(loc, &name, v))
    }

    /// value = [word] | "(" {space} [word {space+ word}] {space} ")". None if empty.
    fn value(&mut self) -> P<Option<Sexp>> {
        if self.peek() == '(' {
            let loc = self.loc(self.index(0));
            self.advance(1);
            self.skip_spaces();
            let mut words = Vec::new();
            while self.peek() != ')' {
                if !self.is_word_start() {
                    return self.err(format!("expected a word or ')', but found {}", self.describe()));
                }
                words.push(self.word()?);
                if self.skip_spaces() == 0 && self.peek() != ')' {
                    return self.err(format!("expected ')', but found {}", self.describe()));
                }
            }
            self.advance(1);
            if self.is_word_start() {
                return self.err(format!("unexpected {} after ')'", self.describe()));
            }
            return Ok(Some(self.node("array", loc, words)));
        }
        if self.is_word_start() {
            Ok(Some(self.word()?))
        } else {
            Ok(None)
        }
    }

    fn ident(&mut self) -> String {
        let mut name = String::new();
        while is_ident_char(self.peek()) {
            name.push(self.peek());
            self.advance(1);
        }
        name
    }

    fn params(&mut self) -> P<Sexp> {
        let loc = self.loc(self.index(0));
        self.advance(1);
        self.skip_spaces();
        let mut names = Vec::new();
        let mut rest = None;
        if is_ident_start(self.peek()) {
            names.push(s(&self.ident()));
            while self.skip_spaces() > 0 && is_ident_start(self.peek()) {
                names.push(s(&self.ident()));
            }
        }
        if self.peek() == '.' {
            self.advance(1);
            self.skip_spaces();
            if !is_ident_start(self.peek()) {
                return self.err(format!("expected a variable name after ., but found {}", self.describe()));
            }
            rest = Some(s(&self.ident()));
            self.skip_spaces();
        }
        self.expect(')')?;
        let mut parts = vec![Sexp::List(Arc::new(names))];
        parts.extend(rest);
        Ok(self.node("params", loc, parts))
    }

    // ── Command lists, pipelines and simple commands ──

    fn command_list(&mut self, mut defs: Option<Vec<Sexp>>) -> P<Sexp> {
        let mut items = Vec::new();
        loop {
            let p = self.pipeline(defs.take())?;
            self.skip_spaces();
            if self.peek() != '&' {
                items.push(p);
                break;
            }
            self.advance(1);
            self.mark_sep();
            let l = pos_of(&p);
            items.push(self.node("bg", l, vec![p]));
            self.skip_spaces();
            if !self.is_word_start() || self.at_comment() || self.at_terminator() {
                break;
            }
        }
        if items.len() == 1 && items[0].tag() != "bg" {
            return Ok(items.pop().unwrap());
        }
        let l = pos_of(&items[0]);
        Ok(self.node("list", l, items))
    }

    fn pipeline(&mut self, defs: Option<Vec<Sexp>>) -> P<Sexp> {
        let mut cmds = vec![self.simple(defs)?];
        loop {
            self.skip_spaces();
            if self.peek() != '|' {
                break;
            }
            self.advance(1);
            self.mark_sep();
            // After |, spaces and newlines, with comments before the newlines.
            loop {
                if self.at_comment() {
                    self.skip_comment();
                }
                if self.at_newline() {
                    self.consume_newline();
                } else if self.skip_spaces() == 0 {
                    break;
                }
            }
            cmds.push(self.simple(None)?);
        }
        if cmds.len() == 1 {
            return Ok(cmds.pop().unwrap());
        }
        let l = pos_of(&cmds[0]);
        Ok(self.node("pipe", l, cmds))
    }

    fn redirs_node(&self, redirs: Vec<Sexp>) -> Sexp {
        let l = pos_of(&redirs[0]);
        self.node("redirs", l, redirs)
    }

    fn simple(&mut self, defs: Option<Vec<Sexp>>) -> P<Sexp> {
        // A simple command starts at the definitions before it, if any.
        let given = defs.is_some();
        let loc = match &defs {
            Some(d) if !d.is_empty() => pos_of(&d[0]),
            _ => self.loc(self.index(0)),
        };
        let defs = match defs {
            Some(d) => d,
            None => {
                let mut d = Vec::new();
                while self.at_def() {
                    d.push(self.def()?);
                    if self.skip_spaces() == 0 {
                        break;
                    }
                }
                d
            }
        };
        let _ = given;
        // A keyword at the start; not after definitions.
        let kw = if defs.is_empty() { self.keyword() } else { None };
        if kw == Some("if") {
            let mut parts = self.if_clauses()?;
            let mut redirs = Vec::new();
            if self.skip_spaces() > 0 && !self.at_comment() && self.at_redirect() {
                self.redirects(&mut redirs)?;
            }
            if !redirs.is_empty() {
                parts.push(self.redirs_node(redirs));
            }
            return Ok(self.node("if", loc, parts));
        }
        if kw == Some("for") {
            let mut parts = self.for_parts()?;
            let mut redirs = Vec::new();
            if self.skip_spaces() > 0 && !self.at_comment() && self.at_redirect() {
                self.redirects(&mut redirs)?;
            }
            if !redirs.is_empty() {
                parts.push(self.redirs_node(redirs));
            }
            return Ok(self.node("for", loc, parts));
        }
        if let Some(k) = kw {
            return self.err(format!("unexpected {}", k));
        }
        if !self.is_word_start() || self.at_redirect() || self.at_comment() {
            return self.err(format!("expected a command, but found {}", self.describe()));
        }
        let mut words = vec![self.word()?];
        let mut redirs = Vec::new();
        loop {
            if self.skip_spaces() == 0 || self.at_comment() {
                break;
            }
            if self.at_redirect() {
                self.redirects(&mut redirs)?;
                break;
            }
            if !self.is_word_start() {
                break;
            }
            words.push(self.word()?);
        }
        let mut parts = Vec::new();
        if !defs.is_empty() {
            let l = pos_of(&defs[0]);
            parts.push(self.node("env", l, defs));
        }
        let l = pos_of(&words[0]);
        parts.push(self.node("words", l, words));
        if !redirs.is_empty() {
            parts.push(self.redirs_node(redirs));
        }
        Ok(self.node("cmd", loc, parts))
    }

    fn redirects(&mut self, redirs: &mut Vec<Sexp>) -> P<()> {
        loop {
            redirs.push(self.redirect()?);
            if self.skip_spaces() == 0 || self.at_comment() {
                break;
            }
            if self.at_redirect() {
                continue;
            }
            if self.is_word_start() {
                return self.err("a word after a redirection".into());
            }
            break;
        }
        Ok(())
    }

    /// "for" space+ name space+ "in" {space+ word} {space} (";" | newline) {blank} "do" script "done"
    fn for_parts(&mut self) -> P<Vec<Sexp>> {
        self.advance(3);
        if self.skip_spaces() == 0 || !is_ident_start(self.peek()) {
            return self.err(format!("expected a variable name after for, but found {}", self.describe()));
        }
        let name = self.ident();
        let spaced = self.skip_spaces() > 0;
        let c = self.peek_at(2);
        if !spaced || self.peek_str(2) != "in" || !(c == END || is_space(c) || c == ';' || self.at_newline_at(2)) {
            return self.err(format!("expected in, but found {}", self.describe()));
        }
        let iloc = self.loc(self.index(0));
        self.advance(2);
        let mut words = Vec::new();
        loop {
            let n = self.skip_spaces();
            if matches!(self.peek(), END | ';') || self.at_newline() || self.at_comment() {
                break;
            }
            if n == 0 || !self.is_word_start() {
                return self.err(format!("expected a word, ';' or a newline, but found {}", self.describe()));
            }
            words.push(self.word()?);
        }
        if self.peek() == ';' {
            self.advance(1);
            self.mark_sep();
        } else {
            if self.at_comment() {
                self.skip_comment();
            }
            if !self.at_newline() {
                return self.err(format!("expected ';' or a newline, but found {}", self.describe()));
            }
            self.consume_newline();
        }
        // Spaces, newlines and comments before do.
        loop {
            self.skip_spaces();
            if self.at_comment() {
                self.skip_comment();
            }
            if !self.at_newline() {
                break;
            }
            self.consume_newline();
        }
        self.expect_keyword("do")?;
        let body = self.script_node(None)?;
        self.expect_keyword("done")?;
        Ok(vec![s(&name), self.node("in", iloc, words), body])
    }

    /// The clauses of if and elif, and else.
    fn if_clauses(&mut self) -> P<Vec<Sexp>> {
        let mut parts = Vec::new();
        let mut kw = Some("if");
        while matches!(kw, Some("if") | Some("elif")) {
            let pos = self.expect_keyword(kw.unwrap())?;
            let cond = self.script_node(None)?;
            self.expect_keyword("then")?;
            let body = self.script_node(None)?;
            let l = self.loc(pos);
            parts.push(self.node("clause", l, vec![cond, body]));
            kw = self.keyword();
        }
        if kw == Some("else") {
            let pos = self.expect_keyword("else")?;
            let body = self.script_node(None)?;
            let l = self.loc(pos);
            parts.push(self.node("else", l, vec![body]));
        }
        self.expect_keyword("fi")?;
        Ok(parts)
    }

    fn at_redirect(&self) -> bool {
        let s = self.peek_str(4);
        s.starts_with('<') || s.starts_with('>') || s.starts_with("2>") || s == "1>&2"
    }

    fn redirect(&mut self) -> P<Sexp> {
        let loc = self.loc(self.index(0));
        let st = self.peek_str(4);
        if st == "1>&2" || st == "2>&1" {
            self.advance(4);
            return Ok(self.node("redir", loc, vec![s(&st)]));
        }
        let op = if st.starts_with(">>") {
            ">>"
        } else if st.starts_with("2>") {
            "2>"
        } else if st.starts_with('<') {
            "<"
        } else {
            ">"
        };
        self.advance(op.len());
        self.skip_spaces();
        if op != ">>" && self.peek() == '(' {
            let p = self.params()?;
            return Ok(self.node("redir", loc, vec![s(op), p]));
        }
        if !self.is_word_start() || self.at_comment() {
            return self.err(format!("expected a redirection target, but found {}", self.describe()));
        }
        let w = self.word()?;
        Ok(self.node("redir", loc, vec![s(op), w]))
    }

    // ── Words ──

    fn add_lit(&self, elems: &mut Vec<Sexp>, x: &str, pos: usize) {
        if let Some(last) = elems.last() {
            if last.tag() == "lit" {
                let text = format!("{}{}", last.children()[0].str(), x);
                let l = pos_of(last);
                *elems.last_mut().unwrap() = self.node("lit", l, vec![s(&text)]);
                return;
            }
        }
        elems.push(self.node("lit", self.loc(pos), vec![s(x)]));
    }

    fn word(&mut self) -> P<Sexp> {
        let loc = self.loc(self.index(0));
        let mut elems = Vec::new();
        loop {
            self.skip();
            let epos = self.pos;
            let c = self.peek();
            if c == '\'' {
                let start = self.pos;
                let end = (self.pos + 1..self.text.len()).find(|&i| self.text[i] == '\'');
                let end = match end {
                    Some(e) => e,
                    None => return Err(ParseError { msg: "unterminated '".into(), pos: start }),
                };
                let body: String = self.text[self.pos + 1..end].iter().collect();
                elems.push(self.node("sq", self.loc(epos), vec![s(&body)]));
                self.pos = end + 1;
            } else if c == '"' {
                elems.push(self.dq()?);
            } else if c == '$' && self.peek_at(1) == '\'' {
                elems.push(self.ansi_c()?);
            } else if c == '$' {
                match self.dollar()? {
                    None => {
                        self.add_lit(&mut elems, "$", epos);
                        self.advance(1);
                    }
                    Some(e) => elems.push(e),
                }
            } else if c == '\\' {
                self.advance(1);
                if self.pos >= self.text.len() {
                    return self.err("\\ at the end of input".into());
                }
                let ch = self.text[self.pos].to_string();
                elems.push(self.node("esc", self.loc(epos), vec![s(&ch)]));
                self.pos += 1;
            } else if self.is_bare() {
                self.add_lit(&mut elems, &c.to_string(), epos);
                self.advance(1);
            } else {
                break;
            }
        }
        Ok(self.node("word", loc, elems))
    }

    /// $'...': an sq node with the escapes made into characters. Line
    /// continuations stay inside.
    fn ansi_c(&mut self) -> P<Sexp> {
        let start = self.pos;
        let t = self.text;
        let mut i = start + 2;
        let mut out = String::new();
        let mut raw: Vec<u8> = Vec::new();
        fn flush(out: &mut String, raw: &mut Vec<u8>) {
            if !raw.is_empty() {
                out.push_str(&String::from_utf8_lossy(raw));
                raw.clear();
            }
        }
        loop {
            if i >= t.len() {
                return Err(ParseError { msg: "unterminated $'".into(), pos: start });
            }
            let c = t[i];
            if c == '\'' {
                break;
            }
            if c != '\\' || i + 1 >= t.len() {
                flush(&mut out, &mut raw);
                out.push(c);
                i += 1;
                continue;
            }
            let d = t[i + 1];
            if d == 'x' || d == 'u' {
                let n = if d == 'x' { 2 } else { 4 };
                let mut j = i + 2;
                while j < t.len() && j < i + 2 + n && t[j].is_ascii_hexdigit() {
                    j += 1;
                }
                if j > i + 2 {
                    let hex: String = t[i + 2..j].iter().collect();
                    let code = u32::from_str_radix(&hex, 16).unwrap();
                    if d == 'x' {
                        raw.push(code as u8);
                    } else {
                        flush(&mut out, &mut raw);
                        out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                    }
                    i = j;
                    continue;
                }
            }
            flush(&mut out, &mut raw);
            match d {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                '\\' => out.push('\\'),
                '\'' => out.push('\''),
                '"' => out.push('"'),
                _ => {
                    out.push('\\');
                    out.push(d);
                }
            }
            i += 2;
        }
        flush(&mut out, &mut raw);
        self.pos = i + 1;
        Ok(self.node("sq", self.loc(start), vec![s(&out)]))
    }

    fn dq(&mut self) -> P<Sexp> {
        let start = self.pos;
        self.advance(1);
        let mut elems = Vec::new();
        loop {
            self.skip();
            let epos = self.pos;
            if self.pos >= self.text.len() {
                return Err(ParseError { msg: "unterminated \"".into(), pos: start });
            }
            let c = self.peek();
            if c == '"' {
                self.advance(1);
                return Ok(self.node("dq", self.loc(start), elems));
            }
            if c == '$' {
                match self.dollar()? {
                    None => {
                        self.add_lit(&mut elems, "$", epos);
                        self.advance(1);
                    }
                    Some(e) => elems.push(e),
                }
            } else if c == '\\' {
                self.advance(1);
                if self.pos >= self.text.len() {
                    return Err(ParseError { msg: "unterminated \"".into(), pos: start });
                }
                let ch = self.text[self.pos];
                self.pos += 1;
                let x = if matches!(ch, '"' | '$' | '\\') { ch.to_string() } else { format!("\\{}", ch) };
                self.add_lit(&mut elems, &x, epos);
            } else {
                self.add_lit(&mut elems, &c.to_string(), epos);
                self.advance(1);
            }
        }
    }

    /// A variable reference or a command substitution, or None (the $ is not read).
    fn dollar(&mut self) -> P<Option<Sexp>> {
        let loc = self.loc(self.index(0));
        let c = self.peek_at(1);
        if c == '(' {
            self.advance(2);
            let script = self.script_node(Some(')'))?;
            self.advance(1);
            return Ok(Some(self.node("subst", loc, vec![script])));
        }
        if c == '{' && self.peek_at(2) == '-' {
            // ${-word}: the LazyList of the glob of the word. A word in "..." too.
            self.advance(3);
            if !self.is_word_start() {
                return self.err(format!("expected a pattern after '${{-', but found {}", self.describe()));
            }
            let word = self.word()?;
            if self.peek() != '}' {
                return self.err(format!("expected '}}', but found {}", self.describe()));
            }
            self.advance(1);
            return Ok(Some(self.node("glob", loc, vec![word])));
        }
        if c == '{' {
            let mut k = 2;
            if is_ident_start(self.peek_at(k)) {
                while is_ident_char(self.peek_at(k)) {
                    k += 1;
                }
            } else if is_digit(self.peek_at(k)) {
                while is_digit(self.peek_at(k)) {
                    k += 1;
                }
            } else if self.peek_at(k) == '@' {
                k += 1;
            }
            if k == 2 || !matches!(self.peek_at(k), '}' | '-') {
                return Ok(None);
            }
            let name: String = (2..k).map(|i| self.peek_at(i)).collect();
            self.advance(k);
            let mut parts = vec![s(&name)];
            if self.peek() == '-' {
                // A default value. A word in "..." too.
                let dloc = self.loc(self.index(0));
                self.advance(1);
                let v = self.value()?;
                parts.push(self.node("default", dloc, v.into_iter().collect()));
                if self.peek() != '}' {
                    return self.err(format!("expected '}}', but found {}", self.describe()));
                }
            }
            self.advance(1);
            return Ok(Some(self.node("bvar", loc, parts)));
        }
        if is_ident_start(c) {
            self.advance(1);
            let name = self.ident();
            return Ok(Some(self.node("var", loc, vec![s(&name)])));
        }
        if is_digit(c) || matches!(c, '@' | '#' | '?' | '!' | '$') {
            self.advance(2);
            return Ok(Some(self.node("var", loc, vec![s(&c.to_string())])));
        }
        Ok(None)
    }
}

/// Parse a script. first_line is the number of its first line (for the lines
/// read so far in interactive mode). The error is "line:col: message".
pub fn parse(text: &str, first_line: i64) -> Result<Vec<Sexp>, (String, String)> {
    let chars: Vec<char> = text.chars().collect();
    let lines = Lines::new(&chars, first_line);
    let mut p = Parser { text: &chars, pos: 0, sep_end: 0, lines: &lines };
    p.program().map_err(|e| {
        let (l, c) = lines.at(e.pos);
        (format!("{}:{}: {}", l, c, e.msg), e.msg)
    })
}

/// A syntax error that the next lines might complete (interactive mode).
pub fn incomplete(msg: &str) -> bool {
    msg.contains("end of input") || msg.starts_with("unterminated")
}

/// The S-expression of a node, as consh_parse writes it.
pub fn write(x: &Sexp, out: &mut String) {
    match x {
        Sexp::List(l) => {
            out.push('(');
            for (i, e) in l.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                write(e, out);
            }
            out.push(')');
        }
        Sexp::Str(t) => {
            out.push('"');
            for c in t.chars() {
                match c {
                    '\\' => out.push_str("\\\\"),
                    '"' => out.push_str("\\\""),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        Sexp::Sym(t) => out.push_str(t),
        Sexp::Num(n) => out.push_str(&n.to_string()),
    }
}
