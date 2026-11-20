//! `Rc`-identity bookkeeping: which nodes get anchors and which are aliases.

use ozzyyaml::Value;
use std::collections::HashMap;

type Ident = *const Value;

/// What to write for one visit of a node.
pub(crate) enum Visit {
    /// Write the node normally.
    Plain,
    /// Write the node with this anchor name.
    Anchor(String),
    /// Write an alias to this already-anchored node.
    Alias(String),
}

/// Identity tables for one emission run.
pub(crate) struct Anchors {
    counts: HashMap<Ident, usize>,
    names: HashMap<Ident, String>,
    next: usize,
}

impl Anchors {
    /// Count how often each node (by address) is reachable from `root`.
    pub(crate) fn scan(root: &Value) -> Self {
        let mut counts = HashMap::new();
        count(root, &mut counts);
        Self {
            counts,
            names: HashMap::new(),
            next: 0,
        }
    }

    /// Decide how to write `value`; assigns an anchor name on first sight of
    /// any node reachable more than once, before its children are visited so
    /// that cycles resolve to aliases.
    pub(crate) fn visit(&mut self, value: &Value) -> Visit {
        let id: Ident = value;
        if let Some(name) = self.names.get(&id) {
            return Visit::Alias(name.clone());
        }
        if self.counts.get(&id).copied().unwrap_or(0) > 1 {
            let name = generated_name(self.next);
            self.next += 1;
            self.names.insert(id, name.clone());
            return Visit::Anchor(name);
        }
        Visit::Plain
    }
}

fn count(value: &Value, counts: &mut HashMap<Ident, usize>) {
    let seen = counts.entry(value).or_insert(0);
    *seen += 1;
    if *seen > 1 {
        return;
    }
    match value {
        Value::Sequence(items) => {
            for item in items.borrow().iter() {
                count(item, counts);
            }
        }
        Value::Mapping(entries) => {
            for (k, v) in entries.borrow().iter() {
                count(k, counts);
                count(v, counts);
            }
        }
        _ => {}
    }
}

/// `a`, `b`, … `z`, `aa`, `ab`, … (bijective base 26).
fn generated_name(mut n: usize) -> String {
    let mut out = Vec::new();
    loop {
        out.push(b'a' + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).expect("ascii")
}

#[cfg(test)]
mod tests {
    use super::generated_name;

    #[test]
    fn generated_names_are_bijective_base_26() {
        assert_eq!(generated_name(0), "a");
        assert_eq!(generated_name(25), "z");
        assert_eq!(generated_name(26), "aa");
        assert_eq!(generated_name(27), "ab");
        assert_eq!(generated_name(26 + 26 * 26), "aaa");
    }
}
