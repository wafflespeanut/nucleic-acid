// Tree + HashMap = Homogeneous Trie
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug)]
pub struct Trie<T: Eq + Hash, S> {
    node: HashMap<T, Trie<T, S>>,
    value: Option<S>,
}

impl<T: Eq + Hash, S> Trie<T, S> {
    pub fn new() -> Trie<T, S> {
        Trie {
            value: None,
            node: HashMap::new(),
        }
    }

    pub fn insert<I: Iterator<Item = T>>(&mut self, mut iterator: I, value: S) {
        match iterator.next() {
            Some(thing) => {
                let mut entry = (&mut self.node).entry(thing).or_insert(Trie::new());
                entry.insert(iterator, value);
            },
            None => {
                self.value = Some(value);
            },
        }
    }

    pub fn get<I: Iterator<Item = T>>(&self, iterator: I) -> Option<&S> {
        let mut current_node = self;
        for thing in iterator {
            if let Some(map) = current_node.node.get(&thing) {
                current_node = &map;
            } else {
                return None
            }
        }

        current_node.value.as_ref()
    }
}
