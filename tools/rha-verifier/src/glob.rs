/// Matches a `/`-separated path: `**` is any number of whole segments,
/// including none; `*` is any run of characters inside one segment.
#[must_use]
pub fn glob(pattern: &str, path: &str) -> bool {
    fn segment(p: &[u8], t: &[u8]) -> bool {
        match p.split_first() {
            None => t.is_empty(),
            Some((b'*', rest)) => (0..=t.len()).any(|i| segment(rest, &t[i..])),
            Some((c, rest)) => t.first() == Some(c) && segment(rest, &t[1..]),
        }
    }
    fn walk(p: &[&str], s: &[&str]) -> bool {
        match p.split_first() {
            None => s.is_empty(),
            Some((&"**", rest)) => (0..=s.len()).any(|i| walk(rest, &s[i..])),
            Some((head, rest)) => s.split_first().is_some_and(|(x, tail)| {
                segment(head.as_bytes(), x.as_bytes()) && walk(rest, tail)
            }),
        }
    }
    let p: Vec<&str> = pattern.split('/').collect();
    let s: Vec<&str> = path.split('/').collect();
    walk(&p, &s)
}

#[cfg(test)]
mod tests {
    use super::glob;

    #[test]
    fn globs() {
        assert!(glob("crates/core/**", "crates/core/src/lib.rs"));
        assert!(glob("**", "a"));
        assert!(glob("crates/*/AGENTS.md", "crates/core/AGENTS.md"));
        assert!(!glob("crates/*/AGENTS.md", "crates/core/x/AGENTS.md"));
        assert!(glob(".rha/policy.toml", ".rha/policy.toml"));
    }
}
