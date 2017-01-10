use bincode::SizeLimit;
use bincode::rustc_serialize as serializer;
use fillings::BitsVec;
use num_traits::{Num, NumCast, cast};
use sa::{Output, suffix_array_or_bwt};

use std::fs::File;
use std::path::Path;

// Generate the BWT of input data (calls the given function with the BWT data as it's generated)
pub fn bwt(input: Vec<u8>) -> Vec<u8> {
    match suffix_array_or_bwt(input, /* generate bwt */ true) {
        Output::BWT(v) => v,
        _ => unreachable!(),
    }
}

// Insert (or) Increment a counter at an index
fn insert<T>(vec: &mut BitsVec<usize>, value: T) -> usize
    where T: Num + NumCast + PartialOrd + Copy
{
    let idx = cast(value).unwrap();
    if vec.len() <= idx {
        vec.extend_with_element(idx + 1, 0);
    }

    let old = vec.get(idx);
    vec.set(idx, old + 1);
    old + 1
}

// Takes a frequency map of bytes and generates the index of first occurrence
// of each byte.
fn generate_occurrence_index(map: &mut BitsVec<usize>) {
    let mut idx = 0;
    for i in 0..map.len() {
        let c = map.get(i);
        map.set(i, idx);
        idx += c;
    }
}

// Invert the BWT data (generate the original data)
pub fn ibwt(input: Vec<u8>) -> Vec<u8> {
    // get the byte distribution
    let bits = (input.len().next_power_of_two() - 1).count_ones() as usize;
    let mut map = BitsVec::new(bits);
    for i in &input {
        insert(&mut map, *i);
    }

    generate_occurrence_index(&mut map);

    // generate the LF vector
    let mut lf = vec![0; input.len()];
    for (i, c) in input.iter().enumerate() {
        let val = map.get(*c as usize);
        lf[i] = val;
        map.set(*c as usize, val + 1);
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

#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub struct FMIndex {
    // BW-transformed data
    data: Vec<u8>,
    // forward frequency of each character in the BWT data
    cache: BitsVec<usize>,
    // incremental character frequencies
    occ_map: BitsVec<usize>,
    // LF-mapping for backward search
    lf_vec: BitsVec<usize>,
}

impl FMIndex {
    pub fn new(data: Vec<u8>) -> FMIndex {
        let mut idx = 0;
        // worst case (all bytes are distinct)
        let bits = (data.len().next_power_of_two() - 1).count_ones() as usize;
        let mut map = BitsVec::new(bits);
        let mut count = BitsVec::with_elements(bits, data.len() + 1, 0);
        let mut lf_vec = count.clone();
        let bwt_data = bwt(data);

        // generate the frequency map and forward frequency vector from BWT
        for i in &bwt_data {
            let value = insert(&mut map, *i);
            count.set(idx, value);
            idx += 1;
        }

        generate_occurrence_index(&mut map);

        let mut lf_occ_map = map.clone();
        // generate the LF vector (just like inverting the BWT)
        for (i, c) in bwt_data.iter().enumerate() {
            let idx = *c as usize;
            let val = lf_occ_map.get(idx);
            lf_vec.set(i, val);
            lf_occ_map.set(idx, val + 1);
        }

        let mut i = 0;
        let mut counter = bwt_data.len();
        // Only difference is that we replace the LF indices with the lengths of prefix
        // from a particular position (in other words, the number of times
        // it would take us to get to the start of string).
        for _ in 0..bwt_data.len() {
            let next = lf_vec.get(i);
            lf_vec.set(i, counter);
            i = next;
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
        let mut result = self.occ_map.get(ch as usize);
        if result > 0 {
            result += (0..idx).rev()
                              .find(|&i| self.data[i] == ch)
                              .map(|i| self.cache.get(i) as usize)
                              .unwrap_or(0);
        }

        result
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
            self.lf_vec.get(i) % self.data.len()
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{FMIndex, bwt, ibwt};

    #[test]
    fn test_bwt_and_ibwt() {
        let text = String::from("ATCTAGGAGATCTGAATCTAGTTCAACTAGCTAGATCTAGAGACAGCTAA");
        let bw = bwt(text.as_bytes().to_owned());
        let ibw = ibwt(bw.clone());
        assert_eq!(String::from("AATCGGAGTTGCTTTG\u{0}AGTAGTGATTTTAAGAAAAAACCCCCCTAAAACG"),
                   String::from_utf8(bw).unwrap());
        assert_eq!(text, String::from_utf8(ibw).unwrap());
    }

    #[test]
    fn test_fm_index() {
        let text = String::from("GCGTGCCCAGGGCACTGCCGCTGCAGGCGTAGGCATCGCATCACACGCGT");
        let index = FMIndex::new(text.as_bytes().to_owned());
        let mut result = index.search("TG");
        result.sort();
        assert_eq!(result, vec![3, 15, 21]);
        let mut result = index.search("GCGT");
        result.sort();
        assert_eq!(result, vec![0, 26, 46]);
        assert_eq!(vec![1], index.search("CGTGCCC"));
    }
}
