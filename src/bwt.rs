use bincode::SizeLimit;
use bincode::rustc_serialize as serializer;
use sa::{insert, suffix_array};

use std::fs::File;
use std::path::Path;

// Generate the BWT of input data (calls the given function with the BWT data as it's generated)
pub fn bwt(input: &[u8]) -> Vec<u8> {
    suffix_array(input).into_iter().map(|i| {
        // BWT[i] = S[SA[i] - 1]
        if i == 0 { 0 } else { input[(i - 1) as usize] }
    }).collect()
}

// Takes a frequency map of bytes and generates the index of first occurrence
// of each byte.
fn generate_occurrence_index(map: &mut Vec<u32>) {
    let mut idx = 0;
    for i in 0..map.len() {
        let c = map[i];
        map[i] = idx;
        idx += c;
    }
}

// Invert the BWT data (generate the original data)
pub fn ibwt(input: &[u8]) -> Vec<u8> {
    // get the byte distribution
    let mut map = Vec::new();
    for i in input {
        insert(&mut map, *i);
    }

    generate_occurrence_index(&mut map);

    // generate the LF vector
    let mut lf = vec![0; input.len()];
    for (i, c) in input.iter().enumerate() {
        let byte = *c as usize;
        let val = map[byte];
        lf[i] = val;
        map[byte] = val + 1;
    }

    let mut idx = 0;
    // construct the sequence by traversing through the LF vector
    let mut output = vec![0; input.len()];
    for i in (0..(input.len() - 1)).rev() {
        output[i] = input[idx];
        idx = lf[idx] as usize;
    }

    output.pop();
    output
}

#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub struct FMIndex {
    // BW-transformed data
    data: Vec<u8>,
    // forward frequency of each character in the BWT data
    cache: Vec<u32>,
    // incremental character frequencies
    occ_map: Vec<u32>,
    // LF-mapping for backward search
    lf_vec: Vec<u32>,
}

impl FMIndex {
    pub fn new(data: &[u8]) -> FMIndex {
        FMIndex::new_from_bwt(bwt(data))
    }

    pub fn new_from_bwt(bwt_data: Vec<u8>) -> FMIndex {
        let mut map = Vec::new();
        let mut count = vec![0u32; bwt_data.len()];
        let mut idx = 0;
        // generate the frequency map and forward frequency vector from BWT
        for i in &bwt_data {
            let value = insert(&mut map, *i);
            count[idx] = value;
            idx += 1;
        }

        generate_occurrence_index(&mut map);

        let mut lf_vec = count.clone();
        let mut lf_occ_map = map.clone();
        // generate the LF vector (just like inverting the BWT)
        for (i, c) in bwt_data.iter().enumerate() {
            let idx = *c as usize;
            let val = lf_occ_map[idx];
            lf_vec[i] = val;
            lf_occ_map[idx] = val + 1;
        }

        let mut i = 0;
        let mut counter = bwt_data.len() as u32;
        // Only difference is that we replace the LF indices with the lengths of prefix
        // from a particular position (in other words, the number of times
        // it would take us to get to the start of string).
        for _ in 0..bwt_data.len() {
            let next = lf_vec[i];
            lf_vec[i] = counter;
            i = next as usize;
            counter -= 1;
        }

        FMIndex {
            data: bwt_data,
            cache: count,
            occ_map: map,
            lf_vec: lf_vec,
        }
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<FMIndex, ()> {
        let mut fd = try!(File::open(path).map_err(|_| ()));
        serializer::decode_from(&mut fd, SizeLimit::Infinite).map_err(|_| ())
    }

    pub fn dump<P: AsRef<Path>>(&self, path: P) -> Result<(), ()> {
        let mut fd = try!(File::create(path).map_err(|_| ()));
        serializer::encode_into(&self, &mut fd, SizeLimit::Infinite).map_err(|_| ())
    }

    // Get the index of the nearest occurrence of a character in the BWT data
    fn nearest(&self, idx: usize, ch: u8) -> usize {
        match self.occ_map.get(ch as usize) {
            Some(res) => {
                *res as usize + (0..idx).rev()
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
            self.lf_vec[i] as usize % self.data.len()
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{FMIndex, bwt, ibwt};

    #[test]
    fn test_bwt_and_ibwt() {
        let text = String::from("ATCTAGGAGATCTGAATCTAGTTCAACTAGCTAGATCTAGAGACAGCTAA");
        let bw = bwt(&text.as_bytes().to_owned());
        let ibw = ibwt(&bw);
        assert_eq!(String::from("AATCGGAGTTGCTTTG\u{0}AGTAGTGATTTTAAGAAAAAACCCCCCTAAAACG"),
                   String::from_utf8(bw).unwrap());
        assert_eq!(text, String::from_utf8(ibw).unwrap());
    }

    #[test]
    fn test_fm_index() {
        let text = String::from("GCGTGCCCAGGGCACTGCCGCTGCAGGCGTAGGCATCGCATCACACGCGT");
        let index = FMIndex::new_from_bwt(&text.as_bytes().to_owned());
        let mut result = index.search("TG");
        result.sort();
        assert_eq!(result, vec![3, 15, 21]);
        let mut result = index.search("GCGT");
        result.sort();
        assert_eq!(result, vec![0, 26, 46]);
        assert_eq!(vec![1], index.search("CGTGCCC"));
    }
}
