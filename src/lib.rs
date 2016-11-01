mod trie;

use std::collections::HashMap;

const BASES: &'static str = "ATCGNYRWSKMDVHB";

// 4 bits (per base unit) * 8 units = 32 bits (to store in `u32`)
const BITS_SIZE: u32 = 4;
const NUM_UNITS: usize = 8;

pub use trie::Trie as Trie;

fn shift_bits(chunk: &[u32]) -> u32 {
    let mut shifted = chunk[0];
    for i in 1..chunk.len() {
        shifted = (shifted << BITS_SIZE) | chunk[i];
    }

    shifted
}

#[derive(Debug)]
pub struct SequenceTrie<T> {
    // Underlying Trie with bit-mapped bases. Instead of having individual bases
    // in every level, we have a 32-bit integer that can hold 8 bases at a time
    // which is somewhat space-efficient.
    trie: Trie<u32, T>,
    base_map: HashMap<&'static str, u32>,
}

impl<T> SequenceTrie<T> {
    pub fn new() -> SequenceTrie<T> {
        SequenceTrie {
            trie: Trie::new(),
            base_map: (0..BASES.len()).map(|i| (&BASES[i..(i + 1)], i as u32))
                                      .collect::<HashMap<_, _>>(),
        }
    }

    fn get_mapped_vec(&self, seq: &str) -> Vec<u32> {
        (0..seq.len()).map(|i| {
            let chunk = &seq[i..(i + 1)];
            *self.base_map.get(chunk).unwrap()
        }).collect::<Vec<_>>()
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
