use fillings::{BitsVec, ReprUsize};

use std::collections::HashMap;
use std::mem;
use std::usize;

#[repr(usize)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum SuffixType {
    Small,
    Large,
    LeftMostSmall,
}

impl ReprUsize for SuffixType {
    fn from_usize(i: usize) -> SuffixType {
        unsafe { mem::transmute(i) }
    }

    fn into_usize(self) -> usize { self as usize }
}

fn induced_sort_large(input: &BitsVec<usize>, approx_sa: &mut BitsVec<usize>, marker: usize,
                      mut bucket_heads: BitsVec<usize>, type_map: &BitsVec<SuffixType>) {
    for i in 0..approx_sa.len() {
        let value = approx_sa.get(i);
        if value == marker || value == 0 {
            continue
        }

        let j = value - 1;
        if type_map.get(j) != SuffixType::Large {
            continue    // only the L-types
        }

        let bucket_idx = input.get(j);
        let bucket_value = bucket_heads.get(bucket_idx);
        approx_sa.set(bucket_value, j);
        bucket_heads.set(bucket_idx, bucket_value + 1);
    }
}

fn induced_sort_small(input: &BitsVec<usize>, approx_sa: &mut BitsVec<usize>, marker: usize,
                      mut bucket_tails: BitsVec<usize>, type_map: &BitsVec<SuffixType>) {
    for i in (0..approx_sa.len()).rev() {
        let value = approx_sa.get(i);
        if value == marker || value == 0 {
            continue
        }

        let j = value - 1;
        if type_map.get(j) == SuffixType::Large {
            continue    // only the S-types (and LMS-types as per our grouping)
        }

        let bucket_idx = input.get(j);
        let bucket_value = bucket_tails.get(bucket_idx);
        approx_sa.set(bucket_value, j);
        bucket_tails.set(bucket_idx, bucket_value - 1);
    }
}

// Check whether the string between two LMS bytes have the same lengths and same contents
fn is_equal_lms(input: &BitsVec<usize>, type_map: &BitsVec<SuffixType>, j: usize, k: usize) -> bool {
    if j == input.len() || k == input.len() {
        return false    // null byte
    }

    for i in 0..(input.len() + 1) {
        let first_lms = type_map.get(i + j) == SuffixType::LeftMostSmall;
        let second_lms = type_map.get(i + k) == SuffixType::LeftMostSmall;
        if first_lms && second_lms && i > 0 {
            return true
        } else if (first_lms != second_lms) || (input.get(i + j) != input.get(i + k)) {
            return false
        }
    }

    false
}

// Generates a suffix array and sorts them using the "induced sorting" method
// (Thanks to the python implementation in http://zork.net/~st/jottings/sais.html)
pub fn suffix_array(input: BitsVec<usize>) -> BitsVec<usize> {
    let mut type_map = BitsVec::with_elements(2, input.len() + 1, SuffixType::Small);
    let mut bucket_sizes = HashMap::new();      // byte frequency (FIXME: too costly! should be a vector)

    type_map.set(input.len(), SuffixType::LeftMostSmall);      // null byte
    type_map.set(input.len() - 1, SuffixType::Large);          // should be L-type
    bucket_sizes.insert(input.get(input.len() - 1), 1);

    // 1. Group the bytes into S-type or L-type (also mark LMS types)
    for i in (0..input.len() - 1).rev() {
        let (cur, next) = (input.get(i), input.get(i + 1));
        let mut c = bucket_sizes.entry(cur).or_insert(0);
        *c += 1;

        if cur > next ||
           (cur == next && type_map.get(i + 1) == SuffixType::Large) {
            if type_map.get(i + 1) == SuffixType::Small {
                type_map.set(i + 1, SuffixType::LeftMostSmall);
            }

            type_map.set(i, SuffixType::Large);
        }
    }

    let mut idx = 1;
    let mut bytes = bucket_sizes.keys().map(|i| *i).collect::<Vec<_>>();
    bytes.sort();

    // BitsVec always requires the max number of bits it should hold. Using the size of `usize`
    // would probably render it useless (as it'd be no different from a vector). So, we get the
    // maximum value from our collection (say, MAX), get its size (MAX::bits) and pass it to BitsVec.
    // This way, we can reduce the memory consumed by more than half.
    let max_byte = bytes[bytes.len() - 1];
    // We'll be adding the frequencies, so input.len() would be the worst case
    // (i.e., same character throughout the string)
    let bits = (input.len().next_power_of_two() - 1).count_ones() as usize;
    let mut bucket_heads = BitsVec::with_elements(bits, max_byte + 1, 0);
    let mut bucket_tails = BitsVec::with_elements(bits, max_byte + 1, 0);

    // 2. Fill the bucket heads and tails (heads for L-types and tails for S-types)
    let mut j = 0;
    for i in 0..(max_byte + 1) {
        bucket_heads.set(i, idx);
        if i == bytes[j] {
            idx += bucket_sizes.remove(&bytes[j]).unwrap();
            j += 1;
        }

        bucket_tails.set(i, idx - 1);
    }

    // 3. Build the approximate SA for initial induced sorting
    let input_max = input.len().next_power_of_two() - 1;        // marker for approx. SA
    let input_max_bits = input_max.count_ones() as usize;

    let mut approx_sa = {
        let mut vec = BitsVec::with_elements(input_max_bits, input.len() + 1, input_max);
        let mut bucket_tails = bucket_tails.clone();
        for (i, byte) in input.iter().enumerate() {
            if type_map.get(i) != SuffixType::LeftMostSmall {
                continue        // ignore the L and S types (for now)
            }

            let bucket_idx = byte;
            let bucket_value = bucket_tails.get(bucket_idx);
            vec.set(bucket_value, i);
            bucket_tails.set(bucket_idx, bucket_value - 1);
        }

        vec.set(0, input.len());    // null byte
        vec
    };

    // 4. Induced sort with respect to L & S types (using the buckets)
    induced_sort_large(&input, &mut approx_sa, input_max, bucket_heads.clone(), &type_map);
    induced_sort_small(&input, &mut approx_sa, input_max, bucket_tails.clone(), &type_map);

    // 5. Record the indices that share LMS substrings
    let mut byte = 0;
    let lms_bytes = {
        // Approx SA is no longer needed (it'll be dropped when it goes out of scope)
        let approx_sa = approx_sa;
        let mut last_idx = approx_sa.get(0);
        let mut lms_bytes = BitsVec::with_elements(input_max_bits, input.len() + 1, input.len());
        lms_bytes.set(last_idx, 0);

        for i in 1..approx_sa.len() {
            let count = approx_sa.get(i);
            let idx = if count == input_max { input.len() } else { count };
            if type_map.get(idx) != SuffixType::LeftMostSmall {
                continue
            }

            if !is_equal_lms(&input, &type_map, last_idx, idx) {
                byte += 1;
            }

            last_idx = idx;
            lms_bytes.set(idx, byte);
        }

        lms_bytes
    };

    // ... and filter the unnecessary bytes
    let lms_max_bits = (byte.next_power_of_two() - 1).count_ones() as usize;
    let sum_max_bits = (lms_bytes.len().next_power_of_two() - 1).count_ones() as usize;

    let mut max_byte = 0;
    let mut summary_index = BitsVec::with_capacity(sum_max_bits, lms_bytes.len());
    for (i, b) in lms_bytes.iter().enumerate() {
        if b != input.len() {
            summary_index.push(i);
            if i > max_byte {
                max_byte = i;
            }
        }
    }

    // 6. Build the final SA
    let suffix_max = max_byte.next_power_of_two() - 1;
    let bits = suffix_max.count_ones() as usize;

    let mut final_sa = {        // build the summary SA (using the usable bytes)
        let mut bucket_tails = bucket_tails.clone();
        let summary_sa = if byte == summary_index.len() - 1 {
            let max_bits = (summary_index.len().next_power_of_two() - 1).count_ones() as usize;
            let mut sum_sa = BitsVec::with_elements(max_bits, summary_index.len() + 1, 0);
            sum_sa.set(0, summary_index.len());
            for i in 0..summary_index.len() {
                let idx = lms_bytes.get(summary_index.get(i));
                sum_sa.set(idx + 1, i);
            }

            sum_sa      // recursion begins to unwind (peek of memory consumption)
        } else {
            let mut mapped = BitsVec::with_capacity(lms_max_bits, summary_index.len());
            for i in &summary_index {
                mapped.push(lms_bytes.get(i));
            }

            suffix_array(mapped)
        };

        let mut suffix_idx = BitsVec::with_elements(bits, input.len() + 1, suffix_max);
        for i in (2..summary_sa.len()).rev() {
            let idx = summary_index.get(summary_sa.get(i));
            let bucket_idx = input.get(idx);
            let bucket_value = bucket_tails.get(bucket_idx);
            suffix_idx.set(bucket_value, idx);
            bucket_tails.set(bucket_idx, bucket_value - 1);
        }

        suffix_idx.set(0, input.len());
        suffix_idx
    };

    // ... and sort it one last time
    induced_sort_large(&input, &mut final_sa, suffix_max, bucket_heads, &type_map);
    induced_sort_small(&input, &mut final_sa, suffix_max, bucket_tails, &type_map);

    final_sa
}
