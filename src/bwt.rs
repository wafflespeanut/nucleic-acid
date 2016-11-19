use std::collections::HashMap;

// FIXME: Try the efficient algorithm (takes only O(n) time with additional O(n) space)
// https://de.wikipedia.org/wiki/Suffix-Array-Induced-Sorting
pub fn bwt(mut input: Vec<u8>) -> Vec<u8> {
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
    }

    rots
}

pub fn occurrence_index(input: &[u8]) -> HashMap<u8, usize> {
    // get the byte distribution
    let mut map = HashMap::new();
    for i in input {
        let mut count = map.entry(*i).or_insert(0);
        *count += 1;
    }

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
    let mut occ_map = occurrence_index(&input);

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
    occ_map: HashMap<u8, usize>,
}

impl FMIndex {
    pub fn new(data: Vec<u8>) -> FMIndex {
        let bwt_data = bwt(data);
        FMIndex {
            occ_map: occurrence_index(&bwt_data),
            data: bwt_data,
        }
    }

    fn nearest_lf(&self, idx: usize, ch: u8) -> usize {
        match self.occ_map.get(&ch) {
            Some(occ) => occ + self.data[0..idx].iter().filter(|&i| *i == ch).count(),
            None => 0,
        }
    }

    // FIXME: We need "checkpointed" index!
    pub fn search(&mut self, query: &str) -> Vec<usize> {
        let (mut top, mut bottom) = (0, self.data.len());
        for ch in query.as_bytes().iter().rev() {
            top = self.nearest_lf(top, *ch);
            bottom = self.nearest_lf(bottom, *ch);
        }

        let mut results = vec![];

        for idx in top..bottom {
            let mut pos = 0;
            let mut i = idx;
            while self.data[i] != 0 {
                pos += 1;
                i = self.nearest_lf(i, self.data[i]);
            }

            results.push(pos);
        }

        return results
    }
}
