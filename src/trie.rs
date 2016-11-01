// Tree + HashMap = Homogeneous Trie
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug)]
pub struct Trie<T: Eq + Hash, S> {
    // Since we're using HashMap, it's better if we have the depth
    // as minimum as possible to avoid clutter.
    node: HashMap<T, Trie<T, S>>,
    value: Option<S>,
    // Marker type to remember overwritten values. This is useful when
    // we're interested in unique values. Getting and replacing values is
    // probably a bad idea, since it takes O(n) time.
    pub is_traced_path: bool,
}

impl<T: Eq + Hash, S> Trie<T, S> {
    pub fn new() -> Trie<T, S> {
        Trie {
            value: None,
            node: HashMap::new(),
            is_traced_path: false,
        }
    }

    pub fn insert<I: Iterator<Item = T>>(&mut self, mut iterator: I, value: S) {
        match iterator.next() {
            Some(thing) => {
                let mut entry = (&mut self.node).entry(thing).or_insert(Trie::new());
                entry.insert(iterator, value);
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

    pub fn get<I: Iterator<Item = T>>(&self, iterator: I, check_unique: bool) -> Option<&S> {
        let mut current_node = self;
        for thing in iterator {
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
