use fillings::{BitsVec, ReprUsize};
use num_traits::{Num, NumCast, cast};
use rand::{self, Rng};
use rustc_serialize::{Encodable, Decodable};

use std::mem;
use std::u32;

// Prefer this for marking, instead of Option<u32> (as it requires additional byte of memory)
// We could use usize here, but it will consume a great deal of memory. Keeping that aside, even
// the size of the giant human genome is only 70% of this value (~3 billion bases). So, we're good...
const MARKER: u32 = u32::MAX;

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

fn induced_sort_large<T>(input: &[T], approx_sa: &mut [u32],
                         mut bucket_heads: BitsVec<usize>, type_map: &BitsVec<SuffixType>)
    where T: Num + NumCast + PartialOrd + Copy
{
    for i in 0..approx_sa.len() {
        let byte = approx_sa[i];
        if byte == MARKER || byte == 0 {
            continue
        }

        let j = (byte - 1) as usize;
        if type_map.get(j) != SuffixType::Large {
            continue    // only the L-types
        }

        let bucket_idx = cast(input[j]).unwrap();
        let bucket_value = bucket_heads.get(bucket_idx);
        approx_sa[bucket_value] = byte - 1;
        bucket_heads.set(bucket_idx, bucket_value + 1);
    }
}

fn induced_sort_small<T>(input: &[T], approx_sa: &mut [u32],
                         mut bucket_tails: BitsVec<usize>, type_map: &BitsVec<SuffixType>)
    where T: Num + NumCast + PartialOrd + Copy
{
    for i in (0..approx_sa.len()).rev() {
        let byte = approx_sa[i];
        if byte == MARKER || byte == 0 {
            continue
        }

        let j = (byte - 1) as usize;
        if type_map.get(j) == SuffixType::Large {
            continue    // only the S-types (and LMS-types as per our grouping)
        }

        let bucket_idx = cast(input[j]).unwrap();
        let bucket_value = bucket_tails.get(bucket_idx);
        approx_sa[bucket_value] = byte - 1;
        bucket_tails.set(bucket_idx, bucket_value - 1);
    }
}

// Check whether the string between two LMS bytes have the same lengths and same contents
fn is_equal_lms<T>(input: &[T], type_map: &BitsVec<SuffixType>, j: usize, k: usize) -> bool
    where T: Num + NumCast + PartialOrd + Copy
{
    let length = input.len();
    if j == length || k == length {
        return false    // null byte
    }

    for i in 0..(length + 1) {
        let first_lms = type_map.get(i + j) == SuffixType::LeftMostSmall;
        let second_lms = type_map.get(i + k) == SuffixType::LeftMostSmall;
        if first_lms && second_lms && i > 0 {
            return true
        } else if (first_lms != second_lms) || (input[i + j] != input[i + k]) {
            return false
        }
    }

    false
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

pub enum Output<T: Num + NumCast + PartialOrd + Copy> {
    BWT(Vec<T>),
    SA(Vec<u32>),
}

pub fn suffix_array(input: Vec<u8>) -> Vec<u32> {
    match suffix_array_or_bwt(input, false) {
        Output::SA(v) => v,
        _ => unreachable!(),
    }
}

pub fn suffix_array_or_bwt<T>(input: Vec<T>, bwt: bool) -> Output<T>
    where T: Num + NumCast + PartialOrd + Copy + Encodable + Decodable
{
    let name = rand::thread_rng().gen_ascii_chars().take(10).collect::<String>();
    suffix_array_(input, 0, &name, bwt)
}

// Generates a suffix array and sorts them using the "induced sorting" method
// (Thanks to the python implementation in http://zork.net/~st/jottings/sais.html)
fn suffix_array_<T>(input: Vec<T>, level: usize, name: &str, bwt: bool) -> Output<T>
    where T: Num + NumCast + PartialOrd + Copy + Encodable + Decodable
{
    let length = input.len();
    let length_32 = length as u32;

    let mut type_map = BitsVec::with_elements(2, length + 1, SuffixType::Small);
    // We'll be adding the frequencies, so input.len() would be the worst case
    // (i.e., same character throughout the string)
    let input_marker = length.next_power_of_two() - 1;
    let input_bits = input_marker.count_ones() as usize;
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

    drop(bytes);
    drop(bucket_sizes);

    // 3. Build the approximate SA for initial induced sorting
    let mut approx_sa = {
        let mut vec = vec![MARKER; length + 1];
        let mut bucket_tails = bucket_tails.clone();
        for (i, byte) in input.iter().enumerate() {
            if type_map.get(i) != SuffixType::LeftMostSmall {
                continue        // ignore the L and S types (for now)
            }

            let bucket_idx = cast(*byte).unwrap();
            let bucket_value = bucket_tails.get(bucket_idx);
            vec[bucket_value] = i as u32;
            bucket_tails.set(bucket_idx, bucket_value - 1);
        }

        vec[0] = length_32;     // null byte
        vec
    };

    // 4. Induced sort with respect to L & S types (using the buckets)
    induced_sort_large(&input, &mut approx_sa, bucket_heads.clone(), &type_map);
    induced_sort_small(&input, &mut approx_sa, bucket_tails.clone(), &type_map);

    // 5. Record the indices that share LMS substrings
    let mut label = 0;
    let lms_bytes = {
        // Approx SA is no longer needed (it'll be dropped when it goes out of scope)
        let mut approx_sa = approx_sa;
        let mut last_idx = approx_sa[0] as usize;
        let mut lms_vec = BitsVec::with_elements(input_bits, length + 1, length_32);
        lms_vec.set(last_idx, 0);

        for count in approx_sa.drain(1..) {
            let idx = if count == MARKER { length } else { count as usize };
            if type_map.get(idx) != SuffixType::LeftMostSmall {
                continue
            }

            if !is_equal_lms(&input, &type_map, last_idx, idx) {
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
        if b != length_32 {
            summary_index.push(i);
        }
    }

    // 6. Build the final SA
    let mut final_sa = {
        let summary_sa = if label + 1 < summary_index.len() as u32 {
            // recursion (we don't have enough labels - multiple LMS substrings are same)
            let mapped = summary_index.iter().map(|i| lms_bytes.get(i)).collect::<Vec<_>>();
            drop(lms_bytes);
            match suffix_array_(mapped, level + 1, name, bwt) {
                Output::SA(v) => v,
                _ => unreachable!(),
            }
        } else {
            let mut sum_sa = vec![0; summary_index.len() + 1];
            sum_sa[0] = summary_index.len() as u32;
            for (i, val) in summary_index.iter().enumerate() {
                let idx = lms_bytes.get(val) as usize;
                sum_sa[idx + 1] = i as u32;
            }

            drop(lms_bytes);
            sum_sa      // recursion begins to unwind
        };

        let mut bucket_tails = bucket_tails.clone();
        let mut suffix_idx = vec![MARKER; length + 1];
        for i in (2..summary_sa.len()).rev() {
            let idx = summary_index.get(summary_sa[i] as usize);
            let bucket_idx = cast(input[idx]).unwrap();
            let bucket_value = bucket_tails.get(bucket_idx);
            suffix_idx[bucket_value] = idx as u32;
            bucket_tails.set(bucket_idx, bucket_value - 1);
        }

        suffix_idx[0] = length_32;
        suffix_idx
    };

    // ... and sort it one last time
    induced_sort_large(&input, &mut final_sa, bucket_heads, &type_map);
    induced_sort_small(&input, &mut final_sa, bucket_tails, &type_map);

    if level == 0 && bwt {      // peek of memory consumption
        Output::BWT(
            final_sa.into_iter().map(|i| {
                // BWT[i] = S[SA[i] - 1]
                if i == 0 { cast(0).unwrap() } else { input[(i - 1) as usize] }
            }).collect()
        )
    } else {
        Output::SA(final_sa)
    }
}

#[cfg(test)]
mod tests {
    use super::suffix_array;

    #[test]
    fn test_suffix_array() {
        let text = b"ATCGAATCGAGAGATCATCGAATCGAGATCATCGAAATCATCGAATCGTC";
        let sa = suffix_array(text.iter().map(|i| *i).collect::<Vec<_>>());

        let mut rotations = (0..text.len()).map(|i| &text[i as usize..]).collect::<Vec<_>>();
        rotations.sort();

        assert_eq!(sa.into_iter().skip(1).map(|i| &text[i as usize..]).collect::<Vec<_>>(),
                   rotations);
    }
}
