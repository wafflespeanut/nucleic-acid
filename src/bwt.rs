use std::collections::HashMap;

use sa::suffix_array;

// Generate the BWT of input data (calls the given function with the BWT data as it's generated)
pub fn bwt<F: FnMut(u8)>(input: Vec<u8>, mut f: F) -> Vec<u8> {
    // get the sorted suffix array
    let sa = suffix_array(&input);
    let mut bw = vec![0; sa.len()];

    // BWT[i] = S[SA[i] - 1]
    for i in 0..bw.len() {
        if sa[i] == 0 {
            bw[i] = 0;
        } else {
            bw[i] = input[sa[i] - 1];
        }

        f(bw[i]);     // call the function with the final value
    }

    bw
}

// Takes a frequency map of bytes and generates the index of first occurrence
// of each byte in their sorted form
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

// Invert the BWT data (generate the original data)
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
    // LF-mapping for backward search (FIXME: could be seeked using a file)
    lf_vec: Vec<usize>,
}

impl FMIndex {
    pub fn new(data: Vec<u8>) -> FMIndex {
        let mut idx = 0;
        let mut map = HashMap::new();
        let mut count = vec![0; data.len() + 1];
        let mut lf_vec = vec![0; data.len() + 1];

        // generate the frequency map and forward frequency vector as we transform the data
        let bwt_data = bwt(data, |i| {
            let mut c = map.entry(i).or_insert(0);
            *c += 1;
            count[idx] = *c as u32;
            idx += 1;
        });

        let occ_map = occurrence_index(map);
        let mut lf_occ_map = occ_map.clone();
        // generate the LF vector (just like inverting the BWT)
        for (i, c) in bwt_data.iter().enumerate() {
            let mut val = lf_occ_map.get_mut(&c).unwrap();
            lf_vec[i] = *val;
            *val += 1;
        }

        let mut i = 0;
        let mut counter = bwt_data.len();
        // Only difference is that we replace the LF indices with the lengths of prefix
        // from a particular position (in other words, the number of times
        // it would take us to get to the start of string).
        while bwt_data[i] != 0 {
            let next = lf_vec[i];
            lf_vec[i] = counter;
            i = next;
            counter -= 1;
        }

        FMIndex {
            data: bwt_data,
            cache: count,
            occ_map: occ_map,
            lf_vec: lf_vec,
        }
    }

    // Get the index of the nearest occurrence of a character in the BWT data
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

    // Find the positions of occurrences of sub-string in the original data.
    pub fn search(&self, query: &str) -> Vec<usize> {
        let mut top = 0;
        let mut bottom = self.data.len();
        for ch in query.as_bytes().iter().rev() {
            top = self.nearest(top, *ch);
            bottom = self.nearest(bottom, *ch);
        }

        (top..bottom).map(|idx| {
            let i = self.nearest(idx, self.data[idx]);
            // wrap around on overflow, which usually occurs only for the
            // last index of LF vector (or the first index of original string)
            self.lf_vec[i] % (self.data.len() - 1)
        }).collect()
    }
}
