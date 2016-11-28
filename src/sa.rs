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
pub fn suffix_array(input: Vec<u8>) -> Vec<usize> {
    let mut type_map = vec![SuffixType::Small; input.len() + 1];
    let mut bucket_sizes = [0; 256];
    let mut bucket_heads = vec![0; 256];
    let mut bucket_tails = vec![0; 256];

    type_map[input.len()] = SuffixType::LeftMostSmall;      // assume that this is a null byte
    type_map[input.len() - 1] = SuffixType::Large;          // should be L-type
    bucket_sizes[input[input.len() - 1] as usize] = 1;      // don't forget the last value!

    // group the bytes into S-type or L-type (also mark LMS types)
    for i in (0..input.len() - 1).rev() {
        bucket_sizes[input[i] as usize] += 1;

        if input[i] > input[i + 1] ||
           (input[i] == input[i + 1] && type_map[i + 1] == SuffixType::Large) {
            if type_map[i + 1] == SuffixType::Small {
                type_map[i + 1] = SuffixType::LeftMostSmall;
            }

            type_map[i] = SuffixType::Large;
        }
    }

    let mut idx = 1;
    // fill the bucket heads and tails
    for (i, size) in bucket_sizes.iter().enumerate() {
        bucket_heads[i] = idx;
        idx += *size;
        bucket_tails[i] = idx - 1;
    }

    let mut approx_sa = {       // build the first (approximate) SA
        let mut vec = vec![MARKER; input.len() + 1];
        let mut bucket_tails = bucket_tails.clone();
        for (i, byte) in input.iter().enumerate() {
            if type_map[i] != SuffixType::LeftMostSmall {
                continue        // ignore the L and S types (for now)
            }

            let bucket_idx = *byte as usize;
            vec[bucket_tails[bucket_idx]] = i;
            bucket_tails[bucket_idx] -= 1;
        }

        vec[0] = input.len();       // null byte
        vec
    };

    fn induced_sort_large(input: &[u8], approx_sa: &mut [usize],
                          mut bucket_heads: Vec<usize>, type_map: &[SuffixType]) {
        for i in 0..approx_sa.len() {
            if approx_sa[i] == MARKER || approx_sa[i] == 0 {
                continue
            }

            let j = approx_sa[i] - 1;
            if type_map[j] != SuffixType::Large {
                continue    // only the L-types
            }

            let bucket_idx = input[j] as usize;
            approx_sa[bucket_heads[bucket_idx]] = j;
            bucket_heads[bucket_idx] += 1;
        }
    }

    fn induced_sort_small(input: &[u8], approx_sa: &mut [usize],
                          mut bucket_tails: Vec<usize>, type_map: &[SuffixType]) {
        for i in (0..approx_sa.len()).rev() {
            if approx_sa[i] == MARKER || approx_sa[i] == 0 {
                continue
            }

            let j = approx_sa[i] - 1;
            if type_map[j] == SuffixType::Large {
                continue    // only the S-types (and LMS-types as per our grouping)
            }

            let bucket_idx = input[j] as usize;
            approx_sa[bucket_tails[bucket_idx]] = j;
            bucket_tails[bucket_idx] -= 1;
        }
    }

    induced_sort_large(&input, &mut approx_sa, bucket_heads.clone(), &type_map);
    induced_sort_small(&input, &mut approx_sa, bucket_tails.clone(), &type_map);

    fn is_equal_lms(input: &[u8], type_map: &[SuffixType], j: usize, k: usize) -> bool {
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
        let mut approx_sa = approx_sa;
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

    let summary_index = lms_bytes.iter().enumerate().filter_map(|(i, b)| {
        if *b == input.len() { None } else { Some(i) }
    }).collect::<Vec<_>>();

    let mut final_sa = {
        let mut bucket_tails = bucket_tails.clone();
        let summary_sa = if byte == summary_index.len() - 1 {
            let mut sum_sa = vec![0; summary_index.len() + 1];
            sum_sa[0] = summary_index.len();
            for i in 0..summary_index.len() {
                let idx = lms_bytes[summary_index[i]] as usize;
                sum_sa[idx + 1] = i;
            }

            sum_sa
        } else {
            suffix_array(summary_index.iter().map(|i| lms_bytes[*i] as u8).collect())
        };

        let mut suffix_idx = vec![MARKER; input.len() + 1];
        for i in (2..summary_sa.len()).rev() {
            let idx = summary_index[summary_sa[i]];
            let bucket_idx = input[idx] as usize;
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
