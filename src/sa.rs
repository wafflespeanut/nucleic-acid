use bit_vec::BitVec;
use num_traits::{Num, NumCast, cast};
use rustc_serialize::{Encodable, Decodable};

use std::u32;

// Prefer this for marking, instead of Option<u32> (as it requires additional byte of memory)
// We could use usize here, but it will consume a great deal of memory. Keeping that aside, even
// the size of the giant human genome is only 70% of this value (~3 billion bases). So, we're good...
const MARKER: u32 = u32::MAX;

fn induced_sort_large<T>(input: &[T], approx_sa: &mut [u32],
                         mut bucket_heads: Vec<u32>, type_map: &BitVec)
    where T: Num + NumCast + PartialOrd + Copy
{
    for i in 0..approx_sa.len() {
        let byte = approx_sa[i];
        if byte == MARKER || byte == 0 {
            continue
        }

        let j = (byte - 1) as usize;
        if !type_map.get(j).unwrap() {
            continue    // only the L-types
        }

        let bucket_idx: usize = cast(input[j]).unwrap();
        let bucket_value = bucket_heads[bucket_idx];
        approx_sa[bucket_value as usize] = byte - 1;
        bucket_heads[bucket_idx] += 1;
    }
}

fn induced_sort_small<T>(input: &[T], approx_sa: &mut [u32],
                         mut bucket_tails: Vec<u32>, type_map: &BitVec)
    where T: Num + NumCast + PartialOrd + Copy
{
    for i in (0..approx_sa.len()).rev() {
        let byte = approx_sa[i];
        if byte == MARKER || byte == 0 {
            continue
        }

        let j = (byte - 1) as usize;
        if type_map.get(j).unwrap() {
            continue    // only the S-types
        }

        let bucket_idx: usize = cast(input[j]).unwrap();
        let bucket_value = bucket_tails[bucket_idx];
        approx_sa[bucket_value as usize] = byte - 1;
        bucket_tails[bucket_idx] -= 1;
    }
}

// Check whether the string between two LMS bytes have the same lengths and same contents
fn is_equal_lms<T>(input: &[T], lms_map: &BitVec, j: usize, k: usize) -> bool
    where T: Num + NumCast + PartialOrd + Copy
{
    let length = input.len();
    if j == length || k == length {
        return false    // null byte
    }

    for i in 0..(length + 1) {
        let first_lms = lms_map.get(i + j).unwrap();
        let second_lms = lms_map.get(i + k).unwrap();
        if first_lms && second_lms && i > 0 {
            return true
        } else if (first_lms != second_lms) || (input[i + j] != input[i + k]) {
            return false
        }
    }

    false
}

// Insert (or) Increment a counter at an index
pub fn insert<T>(vec: &mut Vec<u32>, value: T) -> u32
    where T: Num + NumCast + PartialOrd + Copy
{
    let idx = cast(value).unwrap();
    if vec.len() <= idx {
        vec.resize(idx + 1, 0);
    }

    vec[idx] += 1;
    vec[idx]
}

pub enum Output<T: Num + NumCast + PartialOrd + Copy> {
    BWT(Vec<T>),
    SA(Vec<u32>),
}

pub fn suffix_array(input: Vec<u8>) -> Vec<u32> {
    match suffix_array_(input, 0, false) {
        Output::SA(v) => v,
        _ => unreachable!(),
    }
}

// Generates a suffix array and sorts them using the "induced sorting" method
// (Thanks to the python implementation in http://zork.net/~st/jottings/sais.html)
pub fn suffix_array_<T>(input: Vec<T>, level: usize, bwt: bool) -> Output<T>
    where T: Num + NumCast + PartialOrd + Copy + Encodable + Decodable
{
    let length = input.len();
    let length_32 = length as u32;

    let mut type_map = BitVec::from_elem(length + 1, false);    // `false` for S-type and `true` for L-type
    let mut lms_map = BitVec::from_elem(length + 1, false);     // LMS type
    let mut bucket_sizes = Vec::new();      // byte frequency (HashMap will be a killer in recursions)

    lms_map.set(length, true);          // null byte
    type_map.set(length - 1, true);     // should be L-type
    insert(&mut bucket_sizes, input[length - 1]);

    // 1. Group the bytes into S-type or L-type (also mark LMS types)
    for i in (0..length - 1).rev() {
        let prev_type = type_map.get(i + 1).unwrap();
        insert(&mut bucket_sizes, input[i]);

        if input[i] > input[i + 1] ||
           (input[i] == input[i + 1] && prev_type /* L-type */) {
            if !prev_type /* S-type */ {
                lms_map.set(i + 1, true);
            }

            type_map.set(i, true);
        }
    }

    let mut idx = 1;
    let bytes = bucket_sizes.iter().enumerate().filter_map(|(i, c)| {
        if *c == 0 { None } else { Some(i) }
    }).collect::<Vec<_>>();

    let max_byte = bytes[bytes.len() - 1];
    let mut bucket_tails = vec![0u32; max_byte + 1];
    let mut bucket_heads = vec![0u32; max_byte + 1];

    // 2. Fill the bucket heads and tails (heads for L-types and tails for S-types)
    let mut j = 0;
    for i in 0..(max_byte + 1) {
        bucket_heads[i] = idx;
        if i == bytes[j] {
            idx += bucket_sizes[bytes[j]];
            j += 1;
        }

        bucket_tails[i] = idx - 1;
    }

    drop(bytes);
    drop(bucket_sizes);

    // 3. Build the approximate SA for initial induced sorting
    let mut approx_sa = {
        let mut vec = vec![MARKER; length + 1];
        let mut bucket_tails = bucket_tails.clone();
        for (i, byte) in input.iter().enumerate() {
            if !lms_map.get(i).unwrap() {
                continue        // ignore the L and S types (for now)
            }

            let bucket_idx: usize = cast(*byte).unwrap();
            let bucket_value = bucket_tails[bucket_idx];
            vec[bucket_value as usize] = i as u32;
            bucket_tails[bucket_idx] -= 1;
        }

        vec[0] = length_32;     // null byte
        vec
    };

    // 4. Induced sort with respect to L & S types (using the buckets)
    induced_sort_large(&input, &mut approx_sa, bucket_heads.clone(), &type_map);
    induced_sort_small(&input, &mut approx_sa, bucket_tails.clone(), &type_map);

    // 5. Record the indices that share LMS substrings
    let mut label = 0;
    let mut lms_bytes = {
        // Approx SA is no longer needed (it'll be dropped when it goes out of scope)
        let mut approx_sa = approx_sa;
        let mut last_idx = approx_sa[0] as usize;
        let mut lms_vec = vec![length_32; length + 1];
        lms_vec[last_idx] = 0;

        for count in approx_sa.drain(1..) {
            let idx = if count == MARKER { length } else { count as usize };
            if !lms_map.get(idx).unwrap() {
                continue
            }

            if !is_equal_lms(&input, &lms_map, last_idx, idx) {
                label += 1;
            }

            last_idx = idx;
            lms_vec[idx] = label;
        }

        lms_vec
    };

    drop(lms_map);

    // Both these vectors (even if they're combined) are smaller than `lms_bytes`.
    // So, we can filter out the unnecessary bytes and drop `lms_bytes`
    let mut summary_index_idx = Vec::with_capacity(lms_bytes.len() / 2);
    let mut summary_index_val = Vec::with_capacity(lms_bytes.len() / 2);

    for (i, b) in lms_bytes.drain(..).enumerate() {
        if b != length_32 {
            summary_index_idx.push(i as u32);
            summary_index_val.push(b);
        }
    }

    let summary_len = summary_index_idx.len() as u32;
    drop(lms_bytes);

    // 6. Build the final SA
    let mut final_sa = {
        let summary_sa = if label + 1 < summary_len {
            // recursion (we don't have enough labels - multiple LMS substrings are same)
            match suffix_array_(summary_index_val, level + 1, bwt) {
                Output::SA(v) => v,
                _ => unreachable!(),
            }
        } else {
            let summary_index = summary_index_val;
            let mut sum_sa = vec![0u32; summary_index.len() + 1];
            sum_sa[0] = summary_len;
            for (i, idx) in summary_index.into_iter().enumerate() {
                sum_sa[idx as usize + 1] = i as u32;
            }

            sum_sa      // recursion begins to unwind
        };

        let mut bucket_tails = bucket_tails.clone();
        let mut suffix_idx = vec![MARKER; length + 1];
        let summary_index = summary_index_idx;

        for i in (2..summary_sa.len()).rev() {
            let idx = summary_index[summary_sa[i] as usize];
            let bucket_idx: usize = cast(input[idx as usize]).unwrap();
            let bucket_value = bucket_tails[bucket_idx];
            suffix_idx[bucket_value as usize] = idx as u32;
            bucket_tails[bucket_idx] -= 1;
        }

        suffix_idx[0] = length_32;
        suffix_idx
    };

    // ... and sort it one last time
    induced_sort_large(&input, &mut final_sa, bucket_heads, &type_map);
    induced_sort_small(&input, &mut final_sa, bucket_tails, &type_map);

    if level == 0 && bwt {      // peek of memory consumption
        Output::BWT(
            final_sa.drain(..).map(|i| {
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
