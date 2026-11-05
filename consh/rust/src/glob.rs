//! glob, as consh_glob.se. A pattern is a String in which \c stands for the
//! character c. Quoted characters and the leaves of ${-word} are put in with
//! \ before *, ?, [, ], -, ! and \ (escape); bare characters with \ only
//! before \ (escape_active). The other *, ?, [...] are the wildcards.

use std::path::{Path, PathBuf};

pub fn escape(s: &str) -> String {
    escape_chars(s, "*?[]-!\\")
}

pub fn escape_active(s: &str) -> String {
    escape_chars(s, "\\")
}

fn escape_chars(s: &str, chars: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if chars.contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Whether s has a glob character (bare text).
pub fn has_meta(s: &str) -> bool {
    s.contains(['*', '?', '['])
}

/// The paths that match the pattern, relative to cwd as the pattern is.
pub fn expand(pattern: &str, cwd: &Path) -> Vec<String> {
    let comps: Vec<&str> = pattern.split('/').collect();
    let absolute = pattern.starts_with('/');
    let (prefixes, comps) = if absolute { (vec!["/".to_string()], &comps[1..]) } else { (vec![String::new()], &comps[..]) };
    let (paths, multi) = walk(prefixes, comps, false, cwd);
    if multi {
        let mut paths = paths;
        paths.sort();
        paths
    } else {
        paths
    }
}

fn walk(prefixes: Vec<String>, comps: &[&str], multi: bool, cwd: &Path) -> (Vec<String>, bool) {
    if comps.is_empty() {
        return (prefixes, multi);
    }
    if prefixes.is_empty() {
        return (Vec::new(), multi);
    }
    let comp = comps[0];
    let last = comps.len() == 1;
    if comp.is_empty() {
        let prefixes = if last { dirs(prefixes, cwd) } else { prefixes };
        return walk(prefixes, &comps[1..], multi, cwd);
    }
    if is_magic(comp) {
        let multi = multi || prefixes.len() > 1;
        let dotted = comp.starts_with('.');
        let mut out = Vec::new();
        for prefix in &prefixes {
            for name in listdir(&dir(prefix), cwd).unwrap_or_default() {
                if name.starts_with('.') && !dotted {
                    continue;
                }
                if matches(comp, &name) {
                    out.push(join(prefix, &name));
                }
            }
        }
        return walk(out, &comps[1..], multi, cwd);
    }
    let name = unescape(comp);
    let mut out = Vec::new();
    for prefix in &prefixes {
        let exists = if last {
            match listdir(&dir(prefix), cwd) {
                None => false,
                Some(names) => name == "." || name == ".." || names.iter().any(|n| *n == name),
            }
        } else {
            true
        };
        if exists {
            out.push(join(prefix, &name));
        }
    }
    walk(out, &comps[1..], multi, cwd)
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else if prefix.ends_with('/') {
        format!("{}{}", prefix, name)
    } else {
        format!("{}/{}", prefix, name)
    }
}

fn dir(prefix: &str) -> String {
    if prefix.is_empty() {
        ".".to_string()
    } else {
        prefix.to_string()
    }
}

fn dirs(prefixes: Vec<String>, cwd: &Path) -> Vec<String> {
    prefixes.into_iter().filter(|p| listdir(&dir(p), cwd).is_some()).map(|p| join(&p, "")).collect()
}

/// The names in a directory in code point order, or None if it cannot be read.
pub fn listdir(path: &str, cwd: &Path) -> Option<Vec<String>> {
    let full: PathBuf = cwd.join(path);
    let rd = std::fs::read_dir(full).ok()?;
    let mut names = Vec::new();
    for e in rd {
        names.push(e.ok()?.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    Some(names)
}

/// Whether a part of a pattern has a wildcard without \ before it.
fn is_magic(comp: &str) -> bool {
    if !has_meta(comp) {
        return false;
    }
    let c: Vec<char> = comp.chars().collect();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '\\' {
            if i + 2 > c.len() {
                return false;
            }
            i += 2;
            continue;
        }
        if matches!(c[i], '*' | '?' | '[') {
            return true;
        }
        i += 1;
    }
    false
}

fn unescape(p: &str) -> String {
    let c: Vec<char> = p.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '\\' && i + 1 < c.len() {
            out.push(c[i + 1]);
            i += 2;
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    out
}

/// Whether the pattern p matches the whole name n.
pub fn matches(p: &str, n: &str) -> bool {
    let p: Vec<char> = p.chars().collect();
    let n: Vec<char> = n.chars().collect();
    match_at(&p, 0, &n, 0)
}

fn match_at(p: &[char], pi: usize, n: &[char], ni: usize) -> bool {
    if pi == p.len() {
        return ni == n.len();
    }
    let c = p[pi];
    let next = pi + 1;
    if c == '*' {
        let mut i = ni;
        loop {
            if match_at(p, next, n, i) {
                return true;
            }
            if i == n.len() {
                return false;
            }
            i += 1;
        }
    }
    if ni == n.len() {
        return false;
    }
    let ch = n[ni];
    if c == '?' {
        return match_at(p, next, n, ni + 1);
    }
    if c == '[' {
        let (ok, after, closed) = class(p, next, ch);
        if closed {
            return ok && match_at(p, after, n, ni + 1);
        }
    }
    let (lit, after) = if c == '\\' && next < p.len() { (p[next], next + 1) } else { (c, next) };
    lit == ch && match_at(p, after, n, ni + 1)
}

/// Whether ch is in the class that starts at i (after [), the position after
/// its ], and whether it is closed.
fn class(p: &[char], i: usize, ch: char) -> (bool, usize, bool) {
    if i == p.len() {
        return (false, i, false);
    }
    let negate = p[i] == '!';
    let start = if negate { i + 1 } else { i };
    let code = ch as u32;
    let mut found = false;
    let mut j = start;
    loop {
        if j == p.len() {
            return (if negate { !found } else { found }, j, false);
        }
        let mut c = p[j];
        let mut next = j + 1;
        if c == ']' && j != start {
            return (if negate { !found } else { found }, next, true);
        }
        if c == '\\' && next < p.len() {
            c = p[next];
            next += 1;
        }
        let low = c as u32;
        // A range c-d (if the - is not followed by ]).
        let (high, after) = if next + 2 <= p.len() && p[next] == '-' && p[next + 1] != ']' {
            (p[next + 1] as u32, next + 2)
        } else {
            (prev_code(p, next), next)
        };
        if low <= code && code <= high {
            found = true;
        }
        j = after;
    }
}

fn prev_code(p: &[char], i: usize) -> u32 {
    p[i - 1] as u32
}
