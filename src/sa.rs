#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum SuffixType {
    Small,
    Large,
    LeftMostSmall,
}

// Generates a suffix array and sorts them using the "induced sorting" method
// (Thanks to the python implementation in http://zork.net/~st/jottings/sais.html)
pub fn suffix_array(mut input: Vec<u8>) -> Vec<usize> {
    input.push(0);
    let mut type_map = vec![SuffixType::Small; input.len()];
    let mut bucket_sizes = [0; 256];
    let mut bucket_heads = vec![0; 256];
    let mut bucket_tails = vec![0; 256];

    for i in (0..input.len() - 1).rev() {
        bucket_sizes[input[i] as usize] += 1;

        if input[i] >= input[i + 1] {
            if type_map[i + 1] == SuffixType::Small {
                type_map[i + 1] = SuffixType::LeftMostSmall;
            }

            type_map[i] = SuffixType::Large;
        }
    }

    input.pop();

    let mut idx = 1;
    for (i, size) in bucket_sizes.iter().enumerate() {
        bucket_heads[i] = idx;
        idx += *size;
        bucket_tails[i] = idx - 1;
    }

    let mut approx_sa = {
        let mut vec = vec![-1i32; input.len() + 1];
        let mut bucket_tails = bucket_tails.clone();
        for i in 0..input.len() {
            if type_map[i] != SuffixType::LeftMostSmall {
                continue
            }

            let bucket_idx = input[i] as usize;
            vec[bucket_tails[bucket_idx]] = i as i32;
            bucket_tails[bucket_idx] -= 1;
        }

        vec[0] = input.len() as i32;    // null byte
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

        for i in 0..input.len() {
            let first_lms = type_map[i + j] == SuffixType::LeftMostSmall;
            let second_lms = type_map[i + k] == SuffixType::LeftMostSmall;
            if i > 0 && first_lms && second_lms {
                return true
            } else if (first_lms != second_lms) || (input[i + j] != input[i + k]) {
                return false
            }
        }

        false
    }

    let mut lms_bytes = vec![-1i32; input.len() + 1];
    let mut byte = 0usize;
    lms_bytes[approx_sa[0] as usize] = 0;
    let mut last_idx = approx_sa[0];

    for i in 1..approx_sa.len() {
        let idx = approx_sa[i];
        if type_map[idx as usize] != SuffixType::LeftMostSmall {
            continue
        }

        if !is_equal_lms(&input, &type_map, last_idx as usize, idx as usize) {
            byte += 1;
        }

        last_idx = idx;
        lms_bytes[idx as usize] = byte as i32;
    }

    drop(approx_sa);

    let mut summary_string = Vec::with_capacity(lms_bytes.len());
    let mut summary_index = Vec::with_capacity(lms_bytes.len());
    for (i, b) in lms_bytes.drain(..).enumerate() {
        if b == -1 {
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
            let bucket_idx = input[idx] as usize;
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
