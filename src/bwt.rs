use std::collections::HashMap;

// FIXME: Try the efficient algorithm (takes only O(n) time with additional O(n) space)
// https://de.wikipedia.org/wiki/Suffix-Array-Induced-Sorting
pub fn bwt<F: FnMut(u8)>(mut input: Vec<u8>, mut f: F) -> Vec<u8> {
    input.push(0);

    // get the sorted suffix array
    let mut rotations = (0..input.len()).map(|i| &input[i..]).collect::<Vec<_>>();
    rotations.sort();
    let mut rots = vec![0; input.len()];

    // BWT[i] = S[SA[i] - 1]
    for i in 0..input.len() {
        if rotations[i].len() == input.len() {
            rots[i] = 0;
        } else {
            rots[i] = input[input.len() - rotations[i].len() - 1];
        }

        f(rots[i]);     // call the function with the final value
    }

    rots
}

pub fn occurrence_index(map: HashMap<u8, usize>) -> HashMap<u8, usize> {
    // sort the bytes and locate the index of their first occurrences
    let mut chars = map.keys().collect::<Vec<_>>();
    chars.sort();

    let mut idx = 0;
    let mut occ_map = HashMap::new();
    for i in chars {
        occ_map.insert(*i, idx);
        idx += *map.get(i).unwrap();
    }

    occ_map
}

pub fn ibwt(input: Vec<u8>) -> Vec<u8> {
    // get the byte distribution
    let mut map = HashMap::new();
    for i in &input {
        let mut count = map.entry(*i).or_insert(0);
        *count += 1;
    }

    let mut occ_map = occurrence_index(map);

    // generate the LF vector
    let mut lf = vec![0; input.len()];
    for (i, c) in input.iter().enumerate() {
        let mut val = occ_map.get_mut(&c).unwrap();
        lf[i] = *val;
        *val += 1;
    }

    let mut idx = 0;
    // construct the sequence by traversing through the LF vector
    let mut output = vec![0; input.len()];
    for i in (0..(input.len() - 1)).rev() {
        output[i] = input[idx];
        idx = lf[idx];
    }

    output.pop();
    output
}

pub struct FMIndex {
    data: Vec<u8>,
    count_cache: Vec<usize>,    // remember the occurrence of each char (caching)
    occ_map: HashMap<u8, usize>,
}

impl FMIndex {
    pub fn new(data: Vec<u8>) -> FMIndex {
        let mut idx = 0;
        let mut map = HashMap::new();
        let mut count = vec![0; data.len() + 1];
        let bwt_data = bwt(data, |i| {
            let mut c = map.entry(i).or_insert(0);
            *c += 1;
            count[idx] = *c;
            idx += 1;
        });

        FMIndex {
            occ_map: occurrence_index(map),
            count_cache: count,
            data: bwt_data,
        }
    }

    fn nearest(&self, idx: usize, ch: u8) -> usize {
        match self.occ_map.get(&ch) {
            Some(occ) => {
                occ + (0..idx).rev()
                              .find(|&i| self.data[i] == ch)
                              .map(|i| self.count_cache[i])
                              .unwrap_or(0)
            },
            None => 0,
        }
    }

    // FIXME: We need "checkpointed" index!
    pub fn search(&mut self, query: &str) -> Option<usize> {
        let (mut top, mut bottom) = (0, self.data.len());
        for ch in query.as_bytes().iter().rev() {
            top = self.nearest(top, *ch);
            bottom = self.nearest(bottom, *ch);
        }

        for idx in top..bottom {
            let mut pos = 0;
            let mut i = idx;
            while self.data[i] != 0 {
                pos += 1;
                i = self.nearest(i, self.data[i]);
            }

            return Some(pos)    // break on first find
        }

        None
    }
}
