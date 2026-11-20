//! conma link <runtime_module> <program_module>: join two modules by the names
//! of their globals (docs/dev/PLATFORM-PLAN-ja.md 4.4, stage P5), and write
//! the module to standard output.
//!
//! A language translated into ConMa (ConJS, SJS) includes a large runtime,
//! which the frontend takes about 1.4 s to pass each time. bin/lang makes the
//! module of the runtime once (the runtime module, from a file of the include
//! lines of the translated program) and the module of the translated program
//! without them (the program module, whose globals that the runtime defines
//! are Undefined), and joins them here. The result is the module the frontend
//! would make of the translated program with its includes, but for the
//! numbers of the globals and the order of the files: the globals stay
//! globals, so the program runs as fast and in as many steps.
//!
//! Joining works on the text of the modules (docs/RESOLVER.md, "Module
//! Output"): the items (F ...), (Uses ...), (Platform ...) and (G id "name"
//! value). A global of the program module keeps the number of the global of
//! the same name in the runtime module, or gets a new number after the last
//! one; the references (G n) in the values of the program module are
//! renumbered. A name that both modules define is an error. The files are
//! those of the program module, then those of the runtime module that it does
//! not have; the features of (Uses ...) and (Platform ...) are those of both.

use std::collections::HashMap;

/// The top-level items of a module text, as slices of it.
fn items(text: &str) -> Result<Vec<&str>, String> {
    let t = text.trim_end();
    let inner = t
        .strip_prefix("(Module")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or("link: not a module")?;
    let b = inner.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b' ' | b'\n' | b'\t' | b'\r' => i += 1,
            b'(' => {
                let start = i;
                let mut depth = 0usize;
                let mut in_str = false;
                loop {
                    if i >= b.len() {
                        return Err("link: an item of the module is not closed".into());
                    }
                    let c = b[i];
                    if in_str {
                        if c == b'\\' {
                            i += 1;
                        } else if c == b'"' {
                            in_str = false;
                        }
                    } else if c == b'"' {
                        in_str = true;
                    } else if c == b'(' {
                        depth += 1;
                    } else if c == b')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    i += 1;
                }
                out.push(&inner[start..=i]);
                i += 1;
            }
            _ => return Err("link: unexpected text in the module".into()),
        }
    }
    Ok(out)
}

/// The String literals of an item, as they are written.
fn strings(item: &str) -> Vec<&str> {
    let b = item.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'"' {
            let start = i;
            i += 1;
            while i < b.len() && b[i] != b'"' {
                if b[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            out.push(&item[start..=i.min(b.len() - 1)]);
        }
        i += 1;
    }
    out
}

struct Global<'a> {
    id: usize,
    name: &'a str,
    value: &'a str,
}

/// (G id "name" value): the id, the name as written and the value.
fn global(item: &str) -> Result<Global<'_>, String> {
    let rest = &item[3..item.len() - 1];
    let sp = rest.find(' ').ok_or("link: a bad G item")?;
    let id = rest[..sp].parse::<usize>().map_err(|_| "link: a bad G item")?;
    let rest = &rest[sp + 1..];
    let name = strings(rest).into_iter().next().ok_or("link: a G item without a name")?;
    let value = rest[name.len()..].trim_start();
    Ok(Global { id, name, value })
}

struct Parsed<'a> {
    heads: HashMap<&'a str, &'a str>,
    globals: Vec<Global<'a>>,
}

fn parse(text: &str) -> Result<Parsed<'_>, String> {
    let mut heads = HashMap::new();
    let mut globals = Vec::new();
    for it in items(text)? {
        let tag = it[1..].split(|c: char| c == ' ' || c == ')').next().unwrap_or("");
        if tag == "G" {
            globals.push(global(it)?);
        } else {
            heads.insert(tag, it);
        }
    }
    Ok(Parsed { heads, globals })
}

/// value with each reference (G n) outside String literals renumbered by map.
fn renumber(value: &str, map: &HashMap<usize, usize>) -> String {
    let b = value.as_bytes();
    let mut out = String::with_capacity(value.len() + 16);
    let mut i = 0;
    let mut last = 0;
    while i < b.len() {
        match b[i] {
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
            }
            b'(' if value[i..].starts_with("(G ") => {
                let digits = value[i + 3..].bytes().take_while(|c| c.is_ascii_digit()).count();
                let end = i + 3 + digits;
                if digits > 0 && b.get(end) == Some(&b')') {
                    let n: usize = value[i + 3..end].parse().unwrap();
                    out.push_str(&value[last..i]);
                    out.push_str(&format!("(G {})", map.get(&n).copied().unwrap_or(n)));
                    i = end + 1;
                    last = i;
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out.push_str(&value[last..]);
    out
}

/// The module that joins the program module to the runtime module.
pub fn link(runtime: &str, program: &str) -> Result<String, String> {
    let r = parse(runtime)?;
    let p = parse(program)?;
    // name → (id, value)
    let mut by_name: HashMap<&str, (usize, String)> = HashMap::new();
    let mut next = 0;
    for g in &r.globals {
        by_name.insert(g.name, (g.id, g.value.to_string()));
        next = next.max(g.id + 1);
    }
    let mut map = HashMap::new();
    for g in &p.globals {
        let id = match by_name.get(g.name) {
            Some((id, _)) => *id,
            None => {
                let id = next;
                next += 1;
                by_name.insert(g.name, (id, "Undefined".to_string()));
                id
            }
        };
        map.insert(g.id, id);
    }
    for g in &p.globals {
        if g.value == "Undefined" {
            continue;
        }
        let entry = by_name.get_mut(g.name).unwrap();
        if entry.1 != "Undefined" {
            return Err(format!("link: {} is defined in both modules", g.name));
        }
        entry.1 = renumber(g.value, &map);
    }
    let mut out = String::with_capacity(runtime.len() + program.len());
    out.push_str("(Module (F");
    let pf = p.heads.get("F").map(|f| strings(f)).unwrap_or_default();
    let rf = r.heads.get("F").map(|f| strings(f)).unwrap_or_default();
    for f in pf.iter().chain(rf.iter().filter(|f| !pf.contains(f))) {
        out.push(' ');
        out.push_str(f);
    }
    out.push(')');
    for tag in ["Uses", "Platform"] {
        let mut feats: Vec<&str> = Vec::new();
        for h in [&r.heads, &p.heads] {
            if let Some(it) = h.get(tag) {
                for s in strings(it) {
                    if !feats.contains(&s) {
                        feats.push(s);
                    }
                }
            }
        }
        if !feats.is_empty() {
            feats.sort();
            out.push_str(&format!(" ({} {})", tag, feats.join(" ")));
        }
    }
    let mut gs: Vec<(&&str, &(usize, String))> = by_name.iter().collect();
    gs.sort_by_key(|(_, (id, _))| *id);
    for (name, (id, value)) in gs {
        out.push_str(&format!(" (G {} {} {})", id, name, value));
    }
    out.push_str(")\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_by_name() {
        let r = r#"(Module (F "rt.se") (Uses "boolean") (G 0 "Foo" (Function (1 1) (Head (1 2)) (Body (1 3) (Operator (1 3) (Variable (1 3) "main" (G 1)))))) (G 1 "main" Undefined))"#;
        let p = r#"(Module (F "p.se") (Platform "host") (G 0 "main" (Function (1 1) (Head (1 2)) (Body (1 3) (Operator (1 3) (Variable (1 3) "Foo" (G 1))) (OList (1 4) (Literal (1 4) "(G 0)"))))) (G 1 "Foo" Undefined) (G 2 "Bar" Undefined))"#;
        let m = link(r, p).unwrap();
        assert!(m.starts_with(r#"(Module (F "p.se" "rt.se") (Uses "boolean") (Platform "host") (G 0 "Foo""#));
        assert!(m.contains(r#"(G 1 "main" (Function (1 1) (Head (1 2)) (Body (1 3) (Operator (1 3) (Variable (1 3) "Foo" (G 0))) (OList (1 4) (Literal (1 4) "(G 0)")))))"#));
        assert!(m.contains(r#"(G 2 "Bar" Undefined)"#));
    }

    #[test]
    fn defined_twice() {
        let r = r#"(Module (F "a") (G 0 "x" (Literal (1 1) "1")))"#;
        let p = r#"(Module (F "b") (G 0 "x" (Literal (1 1) "2")))"#;
        assert!(link(r, p).is_err());
    }
}
