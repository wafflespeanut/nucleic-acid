use fillings::{BitsVec, ReprUsize};

use std::mem;
use std::usize;

// Prefer this for marking, instead of Option<usize> (as it requires additional byte of memory)
const MARKER: usize = usize::MAX;

#[repr(usize)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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

fn induced_sort_large(input: &[usize], approx_sa: &mut [usize],
                      mut bucket_heads: BitsVec<usize>, type_map: &BitsVec<SuffixType>) {
    for i in 0..approx_sa.len() {
        if approx_sa[i] == MARKER || approx_sa[i] == 0 {
            continue
        }

        let j = approx_sa[i] - 1;
        if type_map.get(j) != SuffixType::Large {
            continue    // only the L-types
        }

        let bucket_idx = input[j];
        let bucket_value = bucket_heads.get(bucket_idx);
        approx_sa[bucket_value] = j;
        bucket_heads.set(bucket_idx, bucket_value + 1);
    }
}

fn induced_sort_small(input: &[usize], approx_sa: &mut [usize],
                      mut bucket_tails: BitsVec<usize>, type_map: &BitsVec<SuffixType>) {
    for i in (0..approx_sa.len()).rev() {
        if approx_sa[i] == MARKER || approx_sa[i] == 0 {
            continue
        }

        let j = approx_sa[i] - 1;
        if type_map.get(j) == SuffixType::Large {
            continue    // only the S-types (and LMS-types as per our grouping)
        }

        let bucket_idx = input[j];
        let bucket_value = bucket_tails.get(bucket_idx);
        approx_sa[bucket_value] = j;
        bucket_tails.set(bucket_idx, bucket_value - 1);
    }
}

// Check whether the string between two LMS bytes have the same lengths and same contents
fn is_equal_lms(input: &[usize], type_map: &BitsVec<SuffixType>, j: usize, k: usize) -> bool {
    if j == input.len() || k == input.len() {
        return false    // null byte
    }

    for i in 0..(input.len() + 1) {
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
fn insert(vec: &mut BitsVec<usize>, idx: usize) {
    if vec.len() <= idx {
        vec.extend_with_element(idx + 1, 0);
    }

    let old = vec.get(idx);
    vec.set(idx, old + 1);
}

// Generates a suffix array and sorts them using the "induced sorting" method
// (Thanks to the python implementation in http://zork.net/~st/jottings/sais.html)
pub fn suffix_array(input: &[usize]) -> Vec<usize> {
    let mut type_map = BitsVec::with_elements(2, input.len() + 1, SuffixType::Small);
    // We'll be adding the frequencies, so input.len() would be the worst case
    // (i.e., same character throughout the string)
    let bits = (input.len().next_power_of_two() - 1).count_ones() as usize;
    let mut bucket_sizes = BitsVec::new(bits);      // byte frequency (HashMap will be a killer in recursions)

    type_map.set(input.len(), SuffixType::LeftMostSmall);      // null byte
    type_map.set(input.len() - 1, SuffixType::Large);          // should be L-type
    insert(&mut bucket_sizes, input[input.len() - 1]);

    // 1. Group the bytes into S-type or L-type (also mark LMS types)
    for i in (0..input.len() - 1).rev() {
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
    let mut bytes = bucket_sizes.iter().enumerate().filter_map(|(i, c)| {
        if c == 0 { None } else { Some(i) }
    }).collect::<Vec<_>>();
    bytes.sort();

    // BitsVec always requires the max number of bits it should hold. Using the size of `usize`
    // would probably render it useless (as it'd be no different from a vector). So, we get the
    // maximum value from our collection (say, MAX), get its size (MAX::bits) and pass it to BitsVec.
    // This way, we can reduce the memory consumed by more than half.
    let max_byte = bytes[bytes.len() - 1];
    let mut bucket_tails = BitsVec::with_elements(bits, max_byte + 1, 0);
    // (bits + 1) would be worst case, since we'll be incrementing the values again in `induced_sort_large`
    let mut bucket_heads = BitsVec::with_elements(bits + 1, max_byte + 1, 0);

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
        let mut vec = vec![MARKER; input.len() + 1];
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

        vec[0] = input.len();       // null byte
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
        let mut last_idx = approx_sa[0];
        let mut lms_vec = BitsVec::with_elements(bits, input.len() + 1, input.len());
        lms_vec.set(last_idx, 0);

        for count in approx_sa.drain(1..) {
            let idx = if count == MARKER { input.len() } else { count };
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
        if b != input.len() {
            summary_index.push(i);
        }
    }

    // 6. Build the final SA
    let mut final_sa = {
        let mut bucket_tails = bucket_tails.clone();
        let summary_sa = if label + 1 < summary_index.len() {
            // recursion (we don't have enough labels - multiple LMS substrings are same)
            let mapped = summary_index.iter().map(|i| lms_bytes.get(i)).collect::<Vec<_>>();
            drop(lms_bytes);
            suffix_array(&mapped)
        } else {
            let mut sum_sa = vec![0; summary_index.len() + 1];
            sum_sa[0] = summary_index.len();
            for i in 0..summary_index.len() {
                let idx = lms_bytes.get(summary_index.get(i));
                sum_sa[idx + 1] = i;
            }

            drop(lms_bytes);
            sum_sa      // recursion begins to unwind (peek of memory consumption)
        };

        let mut suffix_idx = vec![MARKER; input.len() + 1];
        for i in (2..summary_sa.len()).rev() {
            let idx = summary_index.get(summary_sa[i]);
            let bucket_idx = input[idx];
            let bucket_value = bucket_tails.get(bucket_idx);
            suffix_idx[bucket_value] = idx;
            bucket_tails.set(bucket_idx, bucket_value - 1);
        }

        suffix_idx[0] = input.len();
        suffix_idx
    };

    // ... and sort it one last time
    induced_sort_large(&input, &mut final_sa, bucket_heads, &type_map);
    induced_sort_small(&input, &mut final_sa, bucket_tails, &type_map);

    final_sa
}

#[cfg(test)]
mod tests {
    use super::suffix_array;

    #[test]
    fn test_suffix_array() {
        let text = "ATCGAATCGAGAGATCATCGAATCGAGATCATCGAAATCATCGAATCGTC".to_owned();
        let vec = text.chars().map(|i| i as usize).collect::<Vec<_>>();
        let sa = suffix_array(&vec);

        let mut rotations = (0..text.len()).map(|i| &text[i..]).collect::<Vec<_>>();
        rotations.sort();

        assert_eq!(sa.into_iter().skip(1).map(|i| &text[i..]).collect::<Vec<_>>(),
                   rotations);
    }
}
