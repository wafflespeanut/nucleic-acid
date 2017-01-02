//! Tree + HashMap = Homogeneous Trie

use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug)]
/// A Trie based on a HashMap. Since it's based on a HashMap, it's space *inefficient*.
pub struct Trie<T: Eq + Hash, S> {
    // Since we're using HashMap, it's better if we have the depth
    // as minimum as possible to avoid clutter.
    node: HashMap<T, Trie<T, S>>,
    value: Option<S>,
    // Marker type to remember overwritten values. This is useful when
    // we're interested in unique values. Getting and replacing the values is
    // probably a bad idea, since both the operations take O(n) time.
    is_traced_path: bool,
}

impl<T: Eq + Hash, S> Trie<T, S> {
    /// Create a new Trie.
    pub fn new() -> Trie<T, S> {
        Trie {
            value: None,
            node: HashMap::new(),
            is_traced_path: false,
        }
    }

    /// Insert a value into the trie for a given (hashable) key (represented by an iterator)
    pub fn insert<I>(&mut self, mut iter: I, value: S)
        where I: Iterator<Item=T>
    {
        match iter.next() {
            Some(thing) => {
                let mut entry = (&mut self.node).entry(thing).or_insert(Trie::new());
                entry.insert(iter, value);
            },
            None => {   // End of iteration: Mark if there's a value already
                if self.value.is_some() && self.node.is_empty() {
                    self.is_traced_path = true;
                } else {
                    self.value = Some(value);
                }
            },
        }
    }

    /// Get the value (if any) for a given key. It takes an additional argument `check_unique`
    /// which checks whether the value is unique for a key (or whether it's been overwritten).
    pub fn get<I>(&self, iter: I, check_unique: bool) -> Option<&S>
        where I: Iterator<Item=T>
    {
        let mut current_node = self;
        for thing in iter {
            if let Some(trie) = current_node.node.get(&thing) {
                current_node = &trie;
            } else {
                return None
            }
        }

        if check_unique && current_node.is_traced_path {
            None
        } else {
            current_node.value.as_ref()
        }
    }
}
