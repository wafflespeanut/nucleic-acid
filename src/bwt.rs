use sa::{insert, suffix_array};

use std::ops::Index;

/// Generate the [Burrows-Wheeler Transform](https://en.wikipedia.org/wiki/Burrows%E2%80%93Wheeler_transform)
/// of the given input.
///
/// ``` rust
/// let text = String::from("The quick brown fox jumps over the lazy dog");
/// let bw = helix::bwt(text.as_bytes());
/// assert_eq!(String::from("gkynxeser\u{0}l i hhv otTu c uwd rfm ebp qjoooza"),
///            String::from_utf8(bw).unwrap());
/// ```
/// The output can then be used for compression or FM-index'ing.
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

/// Invert the BWT and generate the original data.
///
/// ``` rust
/// let text = String::from("Hello, world!");
/// let bw = helix::bwt(text.as_bytes());
/// let ibw = helix::ibwt(&bw);
/// assert_eq!(text, String::from_utf8(ibw).unwrap());
/// ```
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

/// [Ferragina-Manzini index](https://en.wikipedia.org/wiki/FM-index)
/// (or Full-text index in Minute space) for finding occurrences of substrings
/// in O(1) time.
///
/// ``` rust
/// use helix::FMIndex;
///
/// let text = String::from("GCGTGCCCAGGGCACTGCCGCTGCAGGCGTAGGCATCGCATCACACGCGT");
/// let index = FMIndex::new(text.as_bytes());
///
/// // count the occurrences
/// assert_eq!(0, index.count("CCCCC"));
/// assert_eq!(3, index.count("TG"));
///
/// // ... or get their positions
/// assert_eq!(index.search("GCGT"), vec![46, 26, 0]);
/// ```
///
/// The current implementation of FM-index is a memory killer, since it stores positions
/// of **all bytes** in the given data. For the human genome (~3 GB), it consumed
/// ~27 GB of RAM to build the index (in ~4 mins).
///
/// That said, it still returns the match results in a few microseconds.
#[derive(Clone, Debug, RustcEncodable, RustcDecodable)]
pub struct FMIndex {
    /// BW-transformed data
    data: Vec<u8>,
    /// forward frequency of each character in the BWT data
    cache: Vec<u32>,
    /// incremental character frequencies
    occ_map: Vec<u32>,
    /// LF-mapping for backward search
    lf_vec: Vec<u32>,
}

impl FMIndex {
    /// Generate an FM-index for the input data.
    #[inline]
    pub fn new(data: &[u8]) -> FMIndex {
        FMIndex::new_from_bwt(bwt(data))
    }

    /// Get the reference to the inner BWT data.
    ///
    /// Note that the length of BWT is one more than the length of the actual text,
    /// since it has a null byte to indicate empty string.
    pub fn bwt(&self) -> &[u8] {
        &self.data
    }

    /// Generate the FM-index from the BWT data.
    ///
    /// It's not a good idea to generate FM-index from scratch all the time, especially for large inputs.
    /// This would be very useful when your data is large and remains constant for a while.
    ///
    /// FM-index internally uses BWT, and BWT is generated from the suffix array, which takes a lot of time.
    /// If your input doesn't change, then it's better to get the BWT data (using `bwt` method), write it
    /// to a file and generate the index from that in the future.
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
            lf_vec[i] = lf_occ_map[idx];
            lf_occ_map[idx] += 1;
        }

        let mut i = lf_vec[0] as usize;
        lf_vec[0] = 0;
        let mut counter = bwt_data.len() as u32 - 1;

        // Only difference is that we replace the LF indices with the lengths of prefix
        // from a particular position (in other words, the number of times
        // it would take us to get to the start of string).
        for _ in 0..(bwt_data.len() - 1) {
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

    /// Get the nearest position of a character in the internal BWT data.
    ///
    /// The `count` and `search` methods rely on this method for finding occurrences.
    /// For example, we can do soemthing like this,
    ///
    /// ``` rust
    /// use helix::FMIndex;
    /// let fm = FMIndex::new(b"Hello, Hello, Hello" as &[u8]);
    ///
    /// // initially, the range should be the length of the BWT
    /// let mut top = 0;
    /// let mut bottom = fm.bwt().len();
    /// let query = b"llo";
    ///
    /// // feed the characters in the reverse
    /// for ch in query.iter().rev() {
    ///     top = fm.nearest(top, *ch);
    ///     bottom = fm.nearest(bottom, *ch);
    ///     if top >= bottom {
    ///         return
    ///     }
    /// }
    ///
    /// // If we get a valid range, then everything in that range is a valid match.
    /// // This way, we can get both the count and positions...
    /// assert_eq!(3, bottom - top);
    /// assert_eq!(vec![17, 10, 3], (top..bottom).map(|i| fm[i]).collect::<Vec<_>>());
    /// ```
    ///
    /// This is backward searching. As you feed in the characters along with a position, `nearest` will
    /// give you a new position in the index. Once the range becomes invalid (which happens when the
    /// substring doesn't exist), we can bail out. On the contrary, if the range remains valid after
    /// you've fed in all the characters, then every value within in that range is an occurrence.
    ///
    /// So, this is useful when you want to cache the repeating ranges. With this, you can build your own
    /// count/search functions with caching. It's also useful for making custom approximate matching functions
    /// by backtracking whenever there's an invalid range.
    pub fn nearest(&self, idx: usize, ch: u8) -> usize {
        match self.occ_map.get(ch as usize) {
            Some(res) if *res > 0 => {
                *res as usize + (0..idx).rev()
                                        .find(|&i| self.data[i] == ch)
                                        .map(|i| self.cache[i] as usize)
                                        .unwrap_or(0)
            },
            _ => 0,
        }
    }

    fn get_range(&self, query: &str) -> Option<(usize, usize)> {
        let mut top = 0;
        let mut bottom = self.data.len();
        for ch in query.as_bytes().iter().rev() {
            top = self.nearest(top, *ch);
            bottom = self.nearest(bottom, *ch);
            if top >= bottom {
                return None
            }
        }

        if top >= bottom {
            None
        } else {
            Some((top, bottom))
        }
    }

    /// Count the occurrences of the substring in the original data.
    pub fn count(&self, query: &str) -> usize {
        match self.get_range(query) {
            Some((top, bottom)) => bottom - top,
            None => 0,
        }
    }

    /// Get the positions of occurrences of substring in the original data.
    pub fn search(&self, query: &str) -> Vec<usize> {
        match self.get_range(query) {
            Some((top, bottom)) =>  (top..bottom).map(|idx| {
                let i = self.nearest(idx, self.data[idx]);
                self.lf_vec[i] as usize
            }).collect(),
            None => Vec::new(),
        }
    }
}

impl Index<usize> for FMIndex {
    type Output = u32;

    fn index(&self, i: usize) -> &u32 {
        self.lf_vec.get(i).expect("index out of range")
    }
}

#[cfg(test)]
mod tests {
    use super::{FMIndex, bwt, ibwt};

    #[test]
    fn test_bwt_and_ibwt() {
        let text = String::from("ATCTAGGAGATCTGAATCTAGTTCAACTAGCTAGATCTAGAGACAGCTAA");
        let bw = bwt(text.as_bytes());
        let ibw = ibwt(&bw);
        assert_eq!(String::from("AATCGGAGTTGCTTTG\u{0}AGTAGTGATTTTAAGAAAAAACCCCCCTAAAACG"),
                   String::from_utf8(bw).unwrap());
        assert_eq!(text, String::from_utf8(ibw).unwrap());
    }

    #[test]
    fn test_fm_index() {
        let text = String::from("GCGTGCCCAGGGCACTGCCGCTGCAGGCGTAGGCATCGCATCACACGCGT");
        let index = FMIndex::new(text.as_bytes());
        assert_eq!(0, index.count("CCCCC"));
        let mut result = index.search("TG");
        result.sort();
        assert_eq!(result, vec![3, 15, 21]);
        let mut result = index.search("GCGT");
        result.sort();
        assert_eq!(result, vec![0, 26, 46]);
        assert_eq!(vec![1], index.search("CGTGCCC"));
    }
}
