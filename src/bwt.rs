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

fn occurrence_index(map: HashMap<u8, usize>) -> HashMap<u8, usize> {
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

#[derive(Clone, Debug)]
pub struct FMIndex {
    // BW-transformed data
    data: Vec<u8>,
    // forward frequency of each character in the BWT data
    // (even the (big-ass) human genome doesn't exceed u32::MAX, so it's good for us)
    cache: Vec<u32>,
    // character frequencies
    occ_map: HashMap<u8, usize>,
    // LF-mapping for backward search
    lf_vec: Vec<u32>,
}

impl FMIndex {
    pub fn new(data: Vec<u8>) -> FMIndex {
        let mut idx = 0;
        let mut map = HashMap::new();
        let mut count = vec![0; data.len() + 1];
        let bwt_data = bwt(data, |i| {
            let mut c = map.entry(i).or_insert(0);
            *c += 1;
            count[idx] = *c as u32;
            idx += 1;
        });

        let occ_map = occurrence_index(map);
        let mut lf_occ_map = occ_map.clone();
        let mut lf_vec = vec![0; bwt_data.len()];
        for (i, c) in bwt_data.iter().enumerate() {
            let mut val = lf_occ_map.get_mut(&c).unwrap();
            lf_vec[i] = *val as u32;
            *val += 1;
        }

        FMIndex {
            data: bwt_data,
            cache: count,
            occ_map: occ_map,
            lf_vec: lf_vec,
        }
    }

    fn nearest(&self, idx: usize, ch: u8) -> usize {
        match self.occ_map.get(&ch) {
            Some(occ) => {
                occ + (0..idx).rev()
                              .find(|&i| self.data[i] == ch)
                              .map(|i| self.cache[i] as usize)
                              .unwrap_or(0)
            },
            None => 0,
        }
    }

    pub fn search(&self, query: &str) -> Option<usize> {
        let mut top = 0;
        let mut bottom = self.data.len();
        for ch in query.as_bytes().iter().rev() {
            top = self.nearest(top, *ch);
            bottom = self.nearest(bottom, *ch);
        }

        for idx in top..bottom {
            let mut i = self.nearest(idx, self.data[idx]);
            let mut pos = 1;

            // Basically, we're just doing the inverse BWT
            // FIXME: Do we have to reconstruct the entire prefix?
            while self.data[i] != 0 {
                pos += 1;
                i = self.lf_vec[i] as usize;
            }

            return Some(pos % self.data.len())      // break on first find
        }

        None
    }
}
