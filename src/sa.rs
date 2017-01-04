use bincode::SizeLimit;
use bincode::rustc_serialize as serializer;
use fillings::{BitsVec, ReprUsize};
use num_traits::{Num, NumCast, cast};
use rand::{self, Rng};
use rustc_serialize::{Decodable, Encodable};

use std::fs::{self, File};
use std::marker::PhantomData;
use std::mem;
use std::path::PathBuf;
use std::u32;

/// Prefer this for marking, instead of Option<usize> (as it requires additional byte of memory)
const MARKER: usize = u32::MAX as usize;    // FIXME: Replace the markers with computed max bytes
/// Default working directory
const DEFAULT_WD: &'static str = "/tmp";
/// Input size beyond which we should prefer File I/O for generating suffix array
const INPUT_LIMIT: usize = 16777216;        // 16 MB (which can take up to ~1 GB of RAM without File I/O)

#[repr(usize)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, RustcEncodable, RustcDecodable)]
/// Enum to represent the type of a character in the data
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

/// Increment the counter at an index (It extends with zeros and increments if the index is out of bounds)
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
/// Suffix Array (built by the induced sorting method)
struct SuffixArray {
    input: BitsVec<usize>,
    type_map: BitsVec<SuffixType>,
    bucket_heads: BitsVec<usize>,
    bucket_tails: BitsVec<usize>,
    array: BitsVec<usize>,
    marker: usize,
    temp_array: BitsVec<usize>,
}

impl SuffixArray {
    /// Steps 1-3
    fn build<T, I>(input_iter: I, length: usize) -> SuffixArray
        where I: DoubleEndedIterator<Item=T> + Clone, T: Num + NumCast + PartialOrd + Copy
    {
        let mut iter = input_iter.clone().rev();
        let mut type_map = BitsVec::with_elements(2, length + 1, SuffixType::Small);
        // We'll be adding the frequencies, so input.len() would be the worst case
        // (i.e., same character throughout the string)
        let input_max = length.next_power_of_two() - 1;
        let input_bits = input_max.count_ones() as usize;
        let mut bucket_sizes = BitsVec::new(input_bits);      // byte frequency (HashMap will be a killer!)

        type_map.set(length, SuffixType::LeftMostSmall);      // null byte
        type_map.set(length - 1, SuffixType::Large);          // should be L-type
        let mut last_byte = iter.next().unwrap();
        insert(&mut bucket_sizes, last_byte);

        // 1. Group the bytes into S-type or L-type (also mark LMS types)
        for i in (0..length - 1).rev() {
            let cur_byte = iter.next().unwrap();
            let prev_type = type_map.get(i + 1);
            insert(&mut bucket_sizes, cur_byte);

            if cur_byte > last_byte ||
               (cur_byte == last_byte && prev_type == SuffixType::Large) {
                if prev_type == SuffixType::Small {
                    type_map.set(i + 1, SuffixType::LeftMostSmall);
                }

                type_map.set(i, SuffixType::Large);
            }

            last_byte = cur_byte;
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
        let max_bits = (max_byte.next_power_of_two() - 1).count_ones() as usize;
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
            let mut vec = BitsVec::with_elements(32, length + 1, MARKER);
            let mut bucket_tails = bucket_tails.clone();
            for (i, byte) in input_iter.clone().enumerate() {
                if type_map.get(i) != SuffixType::LeftMostSmall {
                    continue        // ignore the L and S types (for now)
                }

                let bucket_idx = cast(byte).unwrap();
                let bucket_value = bucket_tails.get(bucket_idx);
                vec.set(bucket_value, i);
                bucket_tails.set(bucket_idx, bucket_value - 1);
            }

            vec.set(0, length);     // null byte
            vec
        };

        SuffixArray {
            input: BitsVec::from_iter(max_bits, input_iter.map(|i| cast(i).unwrap())),
            type_map: type_map,
            bucket_heads: bucket_heads,
            bucket_tails: bucket_tails,
            array: approx_sa,
            marker: MARKER,
            temp_array: BitsVec::new(1),    // will be replaced later
        }
    }

    /// Induced sort with respect to the L-type
    fn induced_sort_large(&mut self) {
        let mut bucket_heads = self.bucket_heads.clone();
        for i in 0..self.array.len() {
            let byte = self.array.get(i);
            if byte == self.marker || byte == 0 {
                continue
            }

            let j = byte - 1;
            if self.type_map.get(j) != SuffixType::Large {
                continue    // only the L-types
            }

            let bucket_idx = self.input.get(j);
            let bucket_value = bucket_heads.get(bucket_idx);
            self.array.set(bucket_value, j);
            bucket_heads.set(bucket_idx, bucket_value + 1);
        }
    }

    /// Induced sort with respect to the S-type
    fn induced_sort_small(&mut self) {
        let mut bucket_tails = self.bucket_tails.clone();
        for i in (0..self.array.len()).rev() {
            let byte = self.array.get(i);
            if byte == self.marker || byte == 0 {
                continue
            }

            let j = byte - 1;
            if self.type_map.get(j) == SuffixType::Large {
                continue    // only the S-types (and LMS-types as per our grouping)
            }

            let bucket_idx = self.input.get(j);
            let bucket_value = bucket_tails.get(bucket_idx);
            self.array.set(bucket_value, j);
            bucket_tails.set(bucket_idx, bucket_value - 1);
        }
    }

    /// Check whether the string between two LMS bytes have the same lengths and same contents
    fn is_equal_lms(&self, j: usize, k: usize) -> bool {
        let length = self.input.len();
        if j == length || k == length {
            return false    // null byte
        }

        for i in 0..(length + 1) {
            let first_lms = self.type_map.get(i + j) == SuffixType::LeftMostSmall;
            let second_lms = self.type_map.get(i + k) == SuffixType::LeftMostSmall;
            if first_lms && second_lms && i > 0 {
                return true
            } else if (first_lms != second_lms) || (self.input.get(i + j) != self.input.get(i + k)) {
                return false
            }
        }

        false
    }

    /// Steps 4-5 (fill the vectors necessary for recursion)
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
            let approx_sa = mem::replace(&mut self.array, BitsVec::new(1));
            let mut last_idx = approx_sa.get(0);
            let mut lms_vec = BitsVec::with_elements(input_bits, length + 1, length);
            lms_vec.set(last_idx, 0);

            for count in approx_sa.iter().skip(1) {
                let idx = if count == self.marker { length } else { count };
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
        let mut summary_index = BitsVec::new(32);
        for (i, b) in lms_bytes.iter().enumerate() {
            if b != length {
                summary_index.push(i);
            }
        }

        let is_recursive = label + 1 < summary_index.len();
        let mapped_array = if is_recursive {
            // We don't have enough labels - multiple LMS substrings are same (recursively sort it)
            BitsVec::from_iter(32, summary_index.iter().map(|i| lms_bytes.get(i)))
        } else {
            let mut sum_sa = BitsVec::with_elements(32, summary_index.len() + 1, 0);
            sum_sa.set(0, summary_index.len());
            for (i, val) in summary_index.iter().enumerate() {
                let idx = lms_bytes.get(val);
                sum_sa.set(idx + 1, i);
            }

            sum_sa      // recursion begins to unwind (peek of memory consumption)
        };

        self.array = mapped_array;
        self.temp_array = summary_index;
        is_recursive
    }

    /// Step 6 - Build the final SA from the array (unwinded from recursion)
    fn fix_stacked(&mut self) {
        let mut bucket_tails = self.bucket_tails.clone();
        let mut suffix_idx = BitsVec::with_elements(32, self.input.len() + 1, MARKER);

        {
            let ref summary_sa = self.array;
            let ref summary_index = self.temp_array;
            for i in (2..summary_sa.len()).rev() {
                let idx = summary_index.get(summary_sa.get(i));
                let bucket_idx = self.input.get(idx);
                let bucket_value = bucket_tails.get(bucket_idx);
                suffix_idx.set(bucket_value, idx);
                bucket_tails.set(bucket_idx, bucket_value - 1);
            }
        }

        suffix_idx.set(0, self.input.len());
        self.array = suffix_idx;

        // ... and sort it one last time
        self.induced_sort_large();
        self.induced_sort_small();
    }
}

/// Private trait for representing a stack (memory-based or file-based)
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

/// Type used for dumping the encodable/decodable values into files.
///
/// The files are prefixed with a random name (10-chars ASCII) and suffixed
/// with a count. The count is incremented for each push, which (in this case)
/// indicates the recursion level. It's very useful for the induced-sorting method,
/// because we only need a few vectors at any moment.
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
        let mut fd = File::open(&path).unwrap();
        let data = serializer::decode_from(&mut fd, SizeLimit::Infinite).unwrap();
        fs::remove_file(path).unwrap();
        Some(data)
    }
}

/// A function for generating the suffix array. It chooses memory or file I/O
/// for sorting, depending on the size of the input.
pub fn suffix_array(input: Vec<u8>) -> Vec<usize> {
    if input.len() > INPUT_LIMIT {
        suffix_array_stacked(input, StackDump::new(DEFAULT_WD))
    } else {
        suffix_array_stacked(input, Vec::new())
    }
}

/// A function to build the suffix array, and to take care of the "stacking" along the way.
/// Suffix array generation based on the "induced sorting" method.
///
/// We prefer stack over recursion<sup>[1]</sup> here, because the method obtains the
/// suffix array in O(n) time by allocating additional vectors, all of which require
/// O(n) spaces. Test runs have showed that building the SA for input sizes of ~250 MB
/// have reached a peak of ~4 GB of physical memory (just before unwinding). This will
/// allow us to go beyond that...
///
/// <sup>[1]: Well, all recursions can be replaced with a stack</sup>
fn suffix_array_stacked<T: Stack<SuffixArray>>(input: Vec<u8>, mut stack: T) -> Vec<usize> {
    let length = input.len();
    let mut sa = SuffixArray::build(input.into_iter(), length);
    let mut is_recursive = sa.prepare_for_stacking();

    while is_recursive {
        let input = sa.array.clone();
        let length = input.len();
        stack.push_back(sa);
        let mut next_sa = SuffixArray::build(input.into_iter(), length);
        is_recursive = next_sa.prepare_for_stacking();
        sa = next_sa;
    }

    sa.fix_stacked();
    while let Some(mut next_sa) = stack.pop_back() {
        next_sa.array = sa.array;
        sa = next_sa;
        sa.fix_stacked();
    }

    sa.array.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::suffix_array;

    #[test]
    fn test_suffix_array() {
        let text = b"ATCGAATCGAGAGATCATCGAATCGAGATCATCGAAATCATCGAATCGTC";
        let sa = suffix_array(text.iter().cloned().collect::<Vec<_>>());

        let mut rotations = (0..text.len()).map(|i| &text[i..]).collect::<Vec<_>>();
        rotations.sort();

        assert_eq!(sa.into_iter().skip(1).map(|i| &text[i..]).collect::<Vec<_>>(),
                   rotations);
    }
}
