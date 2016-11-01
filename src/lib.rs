mod trie;

use std::collections::HashMap;

const BASE_UNITS: [(&'static str, u16); 30] = [
    ("A", 0), ("T", 1), ("C", 2), ("G", 3), ("N", 4),
    ("AA", 5), ("AT", 6), ("AC", 7), ("AG", 8), ("AN", 9),
    ("TA", 10), ("TT", 11), ("TC", 12), ("TG", 13), ("TN", 14),
    ("CA", 15), ("CT", 16), ("CC", 17), ("CG", 18), ("CN", 19),
    ("GA", 20), ("GT", 21), ("GC", 22), ("GG", 23), ("GN", 24),
    ("NA", 25), ("NT", 26), ("NC", 27), ("NG", 28), ("NN", 29),
];

// 5 bits (per base unit) * 3 units = 15 bits (to store in `u16`)
const BITS_SIZE: u16 = 5;
const NUM_UNITS: usize = 3;

pub use trie::Trie as Trie;

fn shift_bits(chunk: &[u16]) -> u16 {
    let mut shifted = chunk[0];
    for i in 1..chunk.len() {
        shifted = (shifted << BITS_SIZE) | chunk[i];
    }

    shifted
}

#[derive(Debug)]
pub struct SequenceTrie<T> {
    // Underlying Trie with bit-mapped bases. Instead of having individual bases
    // in every level, we have a 16-bit integer that can hold 6 bases at a time
    // (or 5, if the sequence has an 'odd' length), which is somewhat space-efficient.
    trie: Trie<u16, T>,
    base_map: HashMap<&'static str, u16>,
}

impl<T> SequenceTrie<T> {
    pub fn new() -> SequenceTrie<T> {
        SequenceTrie {
            trie: Trie::new(),
            base_map: BASE_UNITS.iter().cloned().collect(),
        }
    }

    fn get_mapped_vec(&self, seq: &str) -> Vec<u16> {
        let mut vec = (0..(seq.len() / 2)).map(|i| {
            let chunk = &seq[(i * 2)..((i + 1) * 2)];
            *self.base_map.get(chunk).unwrap()
        }).collect::<Vec<_>>();

        if seq.len() % 2 != 0 {
            let base = &seq[(seq.len() - 1)..];
            vec.push(*self.base_map.get(base).unwrap());
        }

        vec
    }

    pub fn insert(&mut self, seq: &str, value: T) {
        let vec = self.get_mapped_vec(seq);
        self.trie.insert(vec.chunks(NUM_UNITS).into_iter().map(shift_bits), value);
    }

    pub fn get(&self, seq: &str) -> Option<&T> {
        let vec = self.get_mapped_vec(seq);
        self.trie.get(vec.chunks(NUM_UNITS).into_iter().map(shift_bits), false)
    }

    pub fn get_unique(&self, seq: &str) -> Option<&T> {
        let vec = self.get_mapped_vec(seq);
        self.trie.get(vec.chunks(NUM_UNITS).into_iter().map(shift_bits), true)
    }
}
