//! Union-find for schematic electrical connectivity.

use std::collections::BTreeMap;

/// Grid-snapped schematic coordinate key (0.001 world unit resolution).
pub type PointKey = (i64, i64);

/// Disjoint-set (union-find) with path compression.
#[derive(Debug, Default, Clone)]
pub struct DisjointSet {
    parent: BTreeMap<PointKey, PointKey>,
}

impl DisjointSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn make(&mut self, x: PointKey) {
        self.parent.entry(x).or_insert(x);
    }

    pub fn find(&mut self, x: PointKey) -> PointKey {
        self.make(x);
        let p = self.parent[&x];
        if p == x {
            return x;
        }
        let root = self.find(p);
        self.parent.insert(x, root);
        root
    }

    pub fn union(&mut self, a: PointKey, b: PointKey) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }
        let (small, large) = if ra <= rb { (ra, rb) } else { (rb, ra) };
        self.parent.insert(large, small);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unions_are_transitive() {
        let mut dsu = DisjointSet::new();
        let a = (0, 0);
        let b = (1, 0);
        let c = (2, 0);
        dsu.union(a, b);
        dsu.union(b, c);
        assert_eq!(dsu.find(a), dsu.find(c));
    }

    #[test]
    fn separate_components_stay_separate() {
        let mut dsu = DisjointSet::new();
        dsu.union((0, 0), (1, 0));
        dsu.union((100, 0), (101, 0));
        assert_ne!(dsu.find((0, 0)), dsu.find((100, 0)));
    }
}
