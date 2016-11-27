use std::usize;

// FIXME: prefer this over Option<usize> (won't require another byte of memory)
const _MARKER: usize = usize::MAX;

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
    let mut bucket_sizes = vec![0; 256];
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
        let mut vec = vec![-1i32; input.len() + 1];
        let mut bucket_tails = bucket_tails.clone();
        for i in 0..input.len() {
            if type_map[i] != SuffixType::LeftMostSmall {
                continue        // ignore the LMS types for now
            }

            let bucket_idx = input[i] as usize;
            vec[bucket_tails[bucket_idx]] = i as i32;
            bucket_tails[bucket_idx] -= 1;
        }

        vec[0] = input.len() as i32;       // null byte
        vec
    };

    fn induced_sort_large(input: &[u8], approx_sa: &mut [i32],
                          mut bucket_heads: Vec<usize>, type_map: &[SuffixType]) {
        for i in 0..approx_sa.len() {
            let j = approx_sa[i] - 1;
            if j < 0 || type_map[j as usize] != SuffixType::Large {
                continue
            }

            let bucket_idx = input[j as usize] as usize;
            approx_sa[bucket_heads[bucket_idx]] = j;
            bucket_heads[bucket_idx] += 1;
        }
    }

    fn induced_sort_small(input: &[u8], approx_sa: &mut [i32],
                          mut bucket_tails: Vec<usize>, type_map: &[SuffixType]) {
        for i in (0..approx_sa.len()).rev() {
            let j = approx_sa[i] - 1;
            if j < 0 || type_map[j as usize] == SuffixType::Large {
                continue
            }

            let bucket_idx = input[j as usize] as usize;
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

    let mut lms_bytes = vec![input.len(); input.len() + 1];
    let mut byte = 0;
    lms_bytes[approx_sa[0] as usize] = 0;
    let mut last_idx = approx_sa[0] as usize;

    for i in 1..approx_sa.len() {
        let idx = if approx_sa[i] == -1 { input.len() } else { approx_sa[i] as usize };
        if type_map[idx] != SuffixType::LeftMostSmall {
            continue
        }

        if !is_equal_lms(&input, &type_map, last_idx, idx) {
            byte += 1;
        }

        last_idx = idx;
        lms_bytes[idx] = byte;
    }

    drop(approx_sa);

    let mut summary_string = Vec::with_capacity(lms_bytes.len());
    let mut summary_index = Vec::with_capacity(lms_bytes.len());
    for (i, b) in lms_bytes.drain(..).enumerate() {
        if b == input.len() {
            continue
        }

        summary_index.push(i);
        summary_string.push(b as u8);
    }

    let mut sa = {
        let mut bucket_tails = bucket_tails.clone();
        let summary_sa = if byte == summary_string.len() - 1 {
            let mut sa = vec![0usize; summary_string.len() + 1];
            sa[0] = summary_string.len();
            for i in 0..summary_string.len() {
                let idx = summary_string[i];
                sa[idx as usize + 1] = i;
            }

            sa
        } else {
            suffix_array(summary_string)
        };

        let mut suffix_idx = vec![-1i32; input.len() + 1];
        for i in (2..summary_sa.len()).rev() {
            let idx = summary_index[summary_sa[i] as usize];
            let bucket_idx = *input.get(idx).unwrap_or(&0) as usize;
            suffix_idx[bucket_tails[bucket_idx]] = idx as i32;
            bucket_tails[bucket_idx] -= 1;
        }

        suffix_idx[0] = input.len() as i32;

        suffix_idx
    };

    induced_sort_large(&input, &mut sa, bucket_heads, &type_map);
    induced_sort_small(&input, &mut sa, bucket_tails, &type_map);

    sa.into_iter().map(|i| i as usize).collect()
}
