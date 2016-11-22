use std::collections::{HashMap, HashSet};

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
pub struct ByteFrequency {
    pub data: Vec<u8>,
    frequency: Vec<usize>,
    distribution: HashMap<u8, u8>,
}

impl ByteFrequency {
    pub fn new(data: Vec<u8>) -> ByteFrequency {
        let mut set = HashSet::new();
        for ch in &data {
            set.insert(*ch);
        }

        ByteFrequency::new_with_distribution(data, set.iter())
    }

    pub fn new_with_distribution<'a, I>(data: Vec<u8>, distribution: I) -> ByteFrequency
        where I: Iterator<Item=&'a u8>
    {
        let mut chars = distribution.filter(|&i| *i != 0).collect::<Vec<_>>();
        let mut map = HashMap::with_capacity(chars.len());
        let mut vec = vec![0; data.len() + chars.len() - data.len() % chars.len()];
        chars.sort();

        for (i, ch) in data.iter().enumerate() {
            {
                let mut count = map.entry(*ch).or_insert(0);
                *count += 1;
            }

            if i % chars.len() == 0 {
                for (j, c) in chars.iter().enumerate() {
                    vec[i + j] = *map.get(c).unwrap_or(&0);
                }
            }
        }

        ByteFrequency {
            data: data,
            distribution: chars.into_iter().enumerate().map(|(i, ch)| (*ch, i as u8)).collect(),
            frequency: vec,
        }
    }

    pub fn get_distribution(&self, idx: usize, ch: u8) -> usize {
        let pos = idx % self.distribution.len();
        let char_idx = match self.distribution.get(&ch) {
            Some(i) => *i as usize,
            None => return 0,
        };

        let mut count = self.frequency[idx - pos + char_idx];
        if pos != 0 {
            count += self.data[(idx.checked_sub(pos).unwrap_or(0))..idx].iter().filter(|&c| *c == ch).count();
        }

        count
    }
}

#[derive(Clone, Debug)]
pub struct FMIndex {
    // forward frequency of each character in the BWT data
    cache: ByteFrequency,
    // character frequencies
    occ_map: HashMap<u8, usize>,
    // LF-mapping for backward search
    lf_vec: Vec<u32>,
}

impl FMIndex {
    pub fn new(data: Vec<u8>) -> FMIndex {
        let mut map = HashMap::new();
        let bwt_data = bwt(data, |i| {
            let mut c = map.entry(i).or_insert(0);
            *c += 1;
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
            cache: ByteFrequency::new_with_distribution(bwt_data, occ_map.keys().into_iter()),
            occ_map: occ_map,
            lf_vec: lf_vec,
        }
    }

    fn nearest(&self, idx: usize, ch: u8) -> usize {
        match self.occ_map.get(&ch) {
            Some(occ) => {
                occ + self.cache.get_distribution(idx, ch)
            },
            None => 0,
        }
    }

    pub fn search(&self, query: &str) -> Option<usize> {
        let mut top = 0;
        let mut bottom = self.cache.data.len();
        for ch in query.as_bytes().iter().rev() {
            top = self.nearest(top, *ch);
            bottom = self.nearest(bottom, *ch);
        }

        for idx in top..bottom {
            let mut i = self.nearest(idx, self.cache.data[idx]);
            let mut pos = 1;
            while self.cache.data[i] != 0 {
                pos += 1;
                i = self.lf_vec[i] as usize;
            }

            return Some(pos % self.cache.data.len())      // break on first find
        }

        None
    }
}
