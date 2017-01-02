use bincode::SizeLimit;
use bincode::rustc_serialize as serializer;
use fillings::{BitsVec, ReprUsize};
use num_traits::{Num, NumCast, cast};
use rand::{self, Rng};
use rustc_serialize::{Decodable, Encodable};

use std::fs::File;
use std::marker::PhantomData;
use std::mem;
use std::path::PathBuf;
use std::usize;

/// Prefer this for marking, instead of Option<usize> (as it requires additional byte of memory)
const MARKER: usize = usize::MAX;
/// Default working directory
const DEFAULT_WD: &'static str = "/tmp";
/// Input size beyond which we should prefer File I/O for generating suffix array
const INPUT_LIMIT: usize = 16777216;        // 16 MB (which can take up to ~1 GB of RAM without File I/O)

#[repr(usize)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, RustcEncodable, RustcDecodable)]
enum SuffixType {
    Small,
    Large,
    LeftMostSmall,
}

impl ReprUsize for SuffixType {
    fn into_usize(self) -> usize { self as usize }
    fn from_usize(i: usize) -> SuffixType {
        unsafe { mem::transmute(i) }
    }
}

// Increment the counter at an index
pub fn insert<T>(vec: &mut BitsVec<usize>, value: T) -> usize
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

#[derive(RustcEncodable, RustcDecodable)]
struct SuffixArray {
    input: Vec<usize>,
    type_map: BitsVec<SuffixType>,
    bucket_heads: BitsVec<usize>,
    bucket_tails: BitsVec<usize>,
    array: Vec<usize>,
    temp_array: BitsVec<usize>,
}

impl SuffixArray {
    /// Steps 1-3
    fn build(input: Vec<usize>) -> SuffixArray {
        let length = input.len();
        let mut type_map = BitsVec::with_elements(2, length + 1, SuffixType::Small);
        // We'll be adding the frequencies, so input.len() would be the worst case
        // (i.e., same character throughout the string)
        let input_bits = (length.next_power_of_two() - 1).count_ones() as usize;
        let mut bucket_sizes = BitsVec::new(input_bits);      // byte frequency (HashMap will be a killer in recursions)

        type_map.set(length, SuffixType::LeftMostSmall);      // null byte
        type_map.set(length - 1, SuffixType::Large);          // should be L-type
        insert(&mut bucket_sizes, input[length - 1]);

        // 1. Group the bytes into S-type or L-type (also mark LMS types)
        for i in (0..length - 1).rev() {
            let prev_type = type_map.get(i + 1);
            insert(&mut bucket_sizes, input[i]);

            if input[i] > input[i + 1] ||
               (input[i] == input[i + 1] && prev_type == SuffixType::Large) {
                if prev_type == SuffixType::Small {
                    type_map.set(i + 1, SuffixType::LeftMostSmall);
                }

                type_map.set(i, SuffixType::Large);
            }
        }

        let mut idx = 1;
        let bytes = bucket_sizes.iter().enumerate().filter_map(|(i, c)| {
            if c == 0 { None } else { Some(i) }
        }).collect::<Vec<_>>();

        // BitsVec always requires the max number of bits it should hold. Using the size of `usize`
        // would probably render it useless (as it'd be no different from a vector). So, we get the
        // maximum value from our collection (say, MAX), get its size (MAX::bits) and pass it to BitsVec.
        // This way, we can reduce the memory consumed by more than half.
        let max_byte = bytes[bytes.len() - 1];
        let mut bucket_tails = BitsVec::with_elements(input_bits, max_byte + 1, 0);
        // (bits + 1) would be worst case, since we'll be incrementing the values again in `induced_sort_large`
        let mut bucket_heads = BitsVec::with_elements(input_bits + 1, max_byte + 1, 0);

        // 2. Fill the bucket heads and tails (heads for L-types and tails for S-types)
        let mut j = 0;
        for i in 0..(max_byte + 1) {
            bucket_heads.set(i, idx);
            if i == bytes[j] {
                idx += bucket_sizes.get(bytes[j]);
                j += 1;
            }

            bucket_tails.set(i, idx - 1);
        }

        // 3. Build the approximate SA for initial induced sorting
        let approx_sa = {
            let mut vec = vec![MARKER; length + 1];
            let mut bucket_tails = bucket_tails.clone();
            for (i, byte) in input.iter().enumerate() {
                if type_map.get(i) != SuffixType::LeftMostSmall {
                    continue        // ignore the L and S types (for now)
                }

                let bucket_idx = *byte;
                let bucket_value = bucket_tails.get(bucket_idx);
                vec[bucket_value] = i;
                bucket_tails.set(bucket_idx, bucket_value - 1);
            }

            vec[0] = length;        // null byte
            vec
        };

        SuffixArray {
            input: input,
            type_map: type_map,
            bucket_heads: bucket_heads,
            bucket_tails: bucket_tails,
            array: approx_sa,
            temp_array: BitsVec::new(1),    // will be replaced later
        }
    }

    fn induced_sort_large(&mut self) {
        let mut bucket_heads = self.bucket_heads.clone();
        for i in 0..self.array.len() {
            if self.array[i] == MARKER || self.array[i] == 0 {
                continue
            }

            let j = self.array[i] - 1;
            if self.type_map.get(j) != SuffixType::Large {
                continue    // only the L-types
            }

            let bucket_idx = self.input[j];
            let bucket_value = bucket_heads.get(bucket_idx);
            self.array[bucket_value] = j;
            bucket_heads.set(bucket_idx, bucket_value + 1);
        }
    }

    fn induced_sort_small(&mut self) {
        let mut bucket_tails = self.bucket_tails.clone();
        for i in (0..self.array.len()).rev() {
            if self.array[i] == MARKER || self.array[i] == 0 {
                continue
            }

            let j = self.array[i] - 1;
            if self.type_map.get(j) == SuffixType::Large {
                continue    // only the S-types (and LMS-types as per our grouping)
            }

            let bucket_idx = self.input[j];
            let bucket_value = bucket_tails.get(bucket_idx);
            self.array[bucket_value] = j;
            bucket_tails.set(bucket_idx, bucket_value - 1);
        }
    }

    // Check whether the string between two LMS bytes have the same lengths and same contents
    fn is_equal_lms(&self, j: usize, k: usize) -> bool {
        if j == self.input.len() || k == self.input.len() {
            return false    // null byte
        }

        for i in 0..(self.input.len() + 1) {
            let first_lms = self.type_map.get(i + j) == SuffixType::LeftMostSmall;
            let second_lms = self.type_map.get(i + k) == SuffixType::LeftMostSmall;
            if first_lms && second_lms && i > 0 {
                return true
            } else if (first_lms != second_lms) || (self.input[i + j] != self.input[i + k]) {
                return false
            }
        }

        false
    }

    /// Steps 4-5
    fn prepare_for_stacking(&mut self) -> bool {
        // 4. Induced sort with respect to L & S types (using the buckets)
        self.induced_sort_large();
        self.induced_sort_small();

        // 5. Record the indices that share LMS substrings
        let mut label = 0;
        let length = self.input.len();
        let input_bits = (self.input.len().next_power_of_two() - 1).count_ones() as usize;
        let lms_bytes = {
            // Approx SA is no longer needed (it'll be dropped when it goes out of scope)
            let mut approx_sa = mem::replace(&mut self.array, Vec::new());
            let mut last_idx = approx_sa[0];
            let mut lms_vec = BitsVec::with_elements(input_bits, length + 1, length);
            lms_vec.set(last_idx, 0);

            for count in approx_sa.drain(1..) {
                let idx = if count == MARKER { length } else { count };
                if self.type_map.get(idx) != SuffixType::LeftMostSmall {
                    continue
                }

                if !self.is_equal_lms(last_idx, idx) {
                    label += 1;
                }

                last_idx = idx;
                lms_vec.set(idx, label);
            }

            lms_vec
        };

        // ... and filter the unnecessary bytes
        let lms_bits = (lms_bytes.len().next_power_of_two() - 1).count_ones() as usize;
        let mut summary_index = BitsVec::new(lms_bits);
        for (i, b) in lms_bytes.iter().enumerate() {
            if b != length {
                summary_index.push(i);
            }
        }

        let is_recursive = label + 1 < summary_index.len();
        let mapped_array = if is_recursive {
            // we don't have enough labels - multiple LMS substrings are same (recursively sort it)
            summary_index.iter().map(|i| lms_bytes.get(i)).collect()
        } else {
            let mut sum_sa = vec![0; summary_index.len() + 1];
            sum_sa[0] = summary_index.len();
            for (i, val) in summary_index.iter().enumerate() {
                let idx = lms_bytes.get(val);
                sum_sa[idx + 1] = i;
            }

            sum_sa      // recursion begins to unwind (peek of memory consumption)
        };

        self.array = mapped_array;
        self.temp_array = summary_index;
        is_recursive
    }

    /// Step 6 - Build the final SA
    fn fix_stacked(&mut self) {
        let mut bucket_tails = self.bucket_tails.clone();
        let mut suffix_idx = vec![MARKER; self.input.len() + 1];

        {
            let ref summary_sa = self.array;
            let ref summary_index = self.temp_array;
            for i in (2..summary_sa.len()).rev() {
                let idx = summary_index.get(summary_sa[i]);
                let bucket_idx = self.input[idx];
                let bucket_value = bucket_tails.get(bucket_idx);
                suffix_idx[bucket_value] = idx;
                bucket_tails.set(bucket_idx, bucket_value - 1);
            }
        }

        suffix_idx[0] = self.input.len();
        self.array = suffix_idx;

        // ... and sort it one last time
        self.induced_sort_large();
        self.induced_sort_small();
    }
}

trait Stack<T> {
    fn push_back(&mut self, value: T);
    fn pop_back(&mut self) -> Option<T>;
}

impl<T> Stack<T> for Vec<T> {
    fn push_back(&mut self, value: T) {
        self.push(value);
    }

    fn pop_back(&mut self) -> Option<T> {
        self.pop()
    }
}

struct StackDump<T: Encodable + Decodable> {
    path: PathBuf,
    name: String,
    count: usize,
    _marker: PhantomData<T>,
}

impl<T: Encodable + Decodable> StackDump<T> {
    fn new(path: &str) -> StackDump<T> {
        StackDump {
            path: PathBuf::from(path),
            name: rand::thread_rng().gen_ascii_chars().take(10).collect(),
            count: 0,
            _marker: PhantomData,
        }
    }
}

impl<T: Encodable + Decodable> Stack<T> for StackDump<T> {
    fn push_back(&mut self, value: T) {
        let mut path = self.path.clone();
        path.push(format!("{}_{}", self.name, self.count));
        let mut fd = File::create(path).unwrap();
        serializer::encode_into(&value, &mut fd, SizeLimit::Infinite).unwrap();
        self.count += 1;
    }

    fn pop_back(&mut self) -> Option<T> {
        if self.count == 0 {
            return None
        }

        self.count -= 1;
        let mut path = self.path.clone();
        path.push(format!("{}_{}", self.name, self.count));
        let mut fd = File::open(path).unwrap();
        Some(serializer::decode_from(&mut fd, SizeLimit::Infinite).unwrap())
    }
}

pub fn suffix_array(input: &[u8]) -> Vec<usize> {
    if input.len() > INPUT_LIMIT {
        suffix_array_stacked(input, StackDump::new(DEFAULT_WD))
    } else {
        suffix_array_stacked(input, Vec::new())
    }
}

fn suffix_array_stacked<T: Stack<SuffixArray>>(input: &[u8], mut stack: T) -> Vec<usize> {
    let mut sa = SuffixArray::build(input.into_iter().map(|i| *i as usize).collect());
    let mut is_recursive = sa.prepare_for_stacking();

    while is_recursive {
        let mut next_sa = SuffixArray::build(sa.array.clone());
        is_recursive = next_sa.prepare_for_stacking();
        stack.push_back(sa);
        sa = next_sa;
    }

    sa.fix_stacked();
    while let Some(mut next_sa) = stack.pop_back() {
        next_sa.array = mem::replace(&mut sa.array, Vec::new());
        sa = next_sa;
        sa.fix_stacked();
    }

    mem::replace(&mut sa.array, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::suffix_array;

    #[test]
    fn test_suffix_array() {
        let text = b"ATCGAATCGAGAGATCATCGAATCGAGATCATCGAAATCATCGAATCGTC";
        let sa = suffix_array(text);

        let mut rotations = (0..text.len()).map(|i| &text[i..]).collect::<Vec<_>>();
        rotations.sort();

        assert_eq!(sa.into_iter().skip(1).map(|i| &text[i..]).collect::<Vec<_>>(),
                   rotations);
    }
}
