use std::collections::HashMap;
use std::usize;

// Prefer this for marking, instead of Option<usize> (as it requires additional byte of memory)
const MARKER: usize = usize::MAX;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum SuffixType {
    Small,
    Large,
    LeftMostSmall,
}

// Generates a suffix array and sorts them using the "induced sorting" method
// (Thanks to the python implementation in http://zork.net/~st/jottings/sais.html)
pub fn suffix_array(input: &[usize]) -> Vec<usize> {
    let mut type_map = vec![SuffixType::Small; input.len() + 1];
    let mut bucket_sizes = HashMap::new();      // byte frequency

    type_map[input.len()] = SuffixType::LeftMostSmall;      // null byte
    type_map[input.len() - 1] = SuffixType::Large;          // should be L-type
    bucket_sizes.insert(input[input.len() - 1], 1);

    // group the bytes into S-type or L-type (also mark LMS types)
    for i in (0..input.len() - 1).rev() {
        let mut c = bucket_sizes.entry(input[i]).or_insert(0);
        *c += 1;

        if input[i] > input[i + 1] ||
           (input[i] == input[i + 1] && type_map[i + 1] == SuffixType::Large) {
            if type_map[i + 1] == SuffixType::Small {
                type_map[i + 1] = SuffixType::LeftMostSmall;
            }

            type_map[i] = SuffixType::Large;
        }
    }

    let mut idx = 1;
    let mut bytes = bucket_sizes.keys().map(|i| *i).collect::<Vec<_>>();
    bytes.sort();

    let max_byte = bytes[bytes.len() - 1];
    let mut bucket_heads = vec![0; max_byte + 1];
    let mut bucket_tails = vec![0; max_byte + 1];

    // fill the bucket heads and tails (heads for L-types and tails for S-types)
    let mut j = 0;
    for i in 0..(max_byte + 1) {
        bucket_heads[i] = idx;
        if i == bytes[j] {
            idx += bucket_sizes.remove(&bytes[j]).unwrap();
            j += 1;
        }

        bucket_tails[i] = idx - 1;
    }

    let mut approx_sa = {       // build the first (approximate) SA
        let mut vec = vec![MARKER; input.len() + 1];
        let mut bucket_tails = bucket_tails.clone();
        for (i, byte) in input.iter().enumerate() {
            if type_map[i] != SuffixType::LeftMostSmall {
                continue        // ignore the L and S types (for now)
            }

            let bucket_idx = *byte;
            vec[bucket_tails[bucket_idx]] = i;
            bucket_tails[bucket_idx] -= 1;
        }

        vec[0] = input.len();       // null byte
        vec
    };

    fn induced_sort_large(input: &[usize], approx_sa: &mut [usize],
                          mut bucket_heads: Vec<usize>, type_map: &[SuffixType]) {
        for i in 0..approx_sa.len() {
            if approx_sa[i] == MARKER || approx_sa[i] == 0 {
                continue
            }

            let j = approx_sa[i] - 1;
            if type_map[j] != SuffixType::Large {
                continue    // only the L-types
            }

            let bucket_idx = input[j];
            approx_sa[bucket_heads[bucket_idx]] = j;
            bucket_heads[bucket_idx] += 1;
        }
    }

    fn induced_sort_small(input: &[usize], approx_sa: &mut [usize],
                          mut bucket_tails: Vec<usize>, type_map: &[SuffixType]) {
        for i in (0..approx_sa.len()).rev() {
            if approx_sa[i] == MARKER || approx_sa[i] == 0 {
                continue
            }

            let j = approx_sa[i] - 1;
            if type_map[j] == SuffixType::Large {
                continue    // only the S-types (and LMS-types as per our grouping)
            }

            let bucket_idx = input[j];
            approx_sa[bucket_tails[bucket_idx]] = j;
            bucket_tails[bucket_idx] -= 1;
        }
    }

    induced_sort_large(&input, &mut approx_sa, bucket_heads.clone(), &type_map);
    induced_sort_small(&input, &mut approx_sa, bucket_tails.clone(), &type_map);

    // Check whether the string between two LMS bytes have the same lengths and same contents
    fn is_equal_lms(input: &[usize], type_map: &[SuffixType], j: usize, k: usize) -> bool {
        if j == input.len() || k == input.len() {
            return false    // null byte
        }

        for i in 0..(input.len() + 1) {
            let first_lms = type_map[i + j] == SuffixType::LeftMostSmall;
            let second_lms = type_map[i + k] == SuffixType::LeftMostSmall;
            if first_lms && second_lms && i > 0 {
                return true
            } else if (first_lms != second_lms) || (input[i + j] != input[i + k]) {
                return false
            }
        }

        false
    }

    let mut byte = 0;
    let lms_bytes = {
        let mut approx_sa = approx_sa;      // we no longer need this
        let mut last_idx = approx_sa[0];
        let mut lms_bytes = vec![input.len(); input.len() + 1];
        lms_bytes[last_idx] = 0;

        for count in approx_sa.drain(1..) {
            let idx = if count == MARKER { input.len() } else { count };
            if type_map[idx] != SuffixType::LeftMostSmall {
                continue
            }

            if !is_equal_lms(&input, &type_map, last_idx, idx) {
                byte += 1;
            }

            last_idx = idx;
            lms_bytes[idx] = byte;
        }

        lms_bytes
    };

    // filter the modified bytes
    let summary_index = lms_bytes.iter().enumerate().filter_map(|(i, b)| {
        if *b == input.len() { None } else { Some(i) }
    }).collect::<Vec<_>>();

    let mut final_sa = {        // build the summary SA (using the usable bytes)
        let mut bucket_tails = bucket_tails.clone();
        let summary_sa = if byte == summary_index.len() - 1 {
            let mut sum_sa = vec![0; summary_index.len() + 1];
            sum_sa[0] = summary_index.len();
            for i in 0..summary_index.len() {
                let idx = lms_bytes[summary_index[i]];
                sum_sa[idx + 1] = i;
            }

            sum_sa
        } else {
            suffix_array(&summary_index.iter().map(|i| lms_bytes[*i]).collect::<Vec<_>>())
        };

        let mut suffix_idx = vec![MARKER; input.len() + 1];
        for i in (2..summary_sa.len()).rev() {
            let idx = summary_index[summary_sa[i]];
            let bucket_idx = input[idx];
            suffix_idx[bucket_tails[bucket_idx]] = idx;
            bucket_tails[bucket_idx] -= 1;
        }

        suffix_idx[0] = input.len();
        suffix_idx
    };

    induced_sort_large(&input, &mut final_sa, bucket_heads, &type_map);
    induced_sort_small(&input, &mut final_sa, bucket_tails, &type_map);

    final_sa
}
