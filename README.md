## nucleic-acid

This Rust library has some of the bioinformatics stuff I'd written for playing with DNA sequences. It has the following implementations:

 - **BWT** - for generating the Burrows-Wheeler Transform (for the given text) using a suffix array (constructed by the induced sorting method with O(n) space in O(n) time).
 - **FM-Index** - It uses the BWT to count/get the occurrences of substrings in O(1) time. This is the backbone of sequence alignment (note that it's unoptimized in terms of memory).
 - **Bits Vector** - DNA sequences are almost always a bunch of ATCGs. Using 2 bits to represent a nucleotide instead of the usual byte (8 bits) could save *a lot* of memory! `BitsVec` provides a generic interface for stuff that have a known bit range.

### Usage

Add this to your `Cargo.toml`

``` toml
nucleic-acid = "0.1"
```

See the [documentation](https://docs.rs/nucleic-acid) for exact usage and detailed examples.

### Motivation

The implementations for BWT and FM-index have already been provided by the awesome [`rust-bio`](http://github.com/rust-bio/rust-bio/) library. But, that's not great for large datasets (~4 GB). This library was written to handle such datasets efficiently.

### Benchmarks

#### `BitsVec`

Note that `BitsVec` is a lot slower compared to `Vec`, because, we can't move pointers by *bits*, and so we gotta do some shifting and bitwise stuff to achieve this. That's at least 7-10 additional operations (per function call) in addition to the pointer read/write. So, **it's slow**.

    bench_1_bits_vec_fill_with_1000_elements  ... bench:       1,961 ns/iter (+/- 142)
    bench_1_bits_vec_get_1000_ints            ... bench:      26,429 ns/iter (+/- 281)
    bench_1_bits_vec_push_1000_ints           ... bench:       8,574 ns/iter (+/- 1,409)
    bench_1_bits_vec_set_1000_ints            ... bench:      31,423 ns/iter (+/- 948)
    bench_22_bits_vec_fill_with_1000_elements ... bench:       1,422 ns/iter (+/- 184)
    bench_22_bits_vec_get_1000_ints           ... bench:      28,098 ns/iter (+/- 458)
    bench_22_bits_vec_push_1000_ints          ... bench:      11,701 ns/iter (+/- 3,853)
    bench_22_bits_vec_set_1000_ints           ... bench:      32,632 ns/iter (+/- 1,032)
    bench_40_bits_vec_fill_with_1000_elements ... bench:       1,941 ns/iter (+/- 123)
    bench_40_bits_vec_get_1000_ints           ... bench:      27,771 ns/iter (+/- 2,613)
    bench_40_bits_vec_push_1000_ints          ... bench:      13,475 ns/iter (+/- 5,716)
    bench_40_bits_vec_set_1000_ints           ... bench:      32,786 ns/iter (+/- 1,649)
    bench_63_bits_vec_fill_with_1000_elements ... bench:       3,078 ns/iter (+/- 273)
    bench_63_bits_vec_get_1000_ints           ... bench:      29,247 ns/iter (+/- 2,903)
    bench_63_bits_vec_push_1000_ints          ... bench:      20,756 ns/iter (+/- 2,717)
    bench_63_bits_vec_set_1000_ints           ... bench:      34,674 ns/iter (+/- 2,819)

As you may notice, this becomes inefficient once you approach the size of `usize` (in my case, pushing 63 bit values is a lot slower than pushing 22 or 40 bit values).

#### `suffix_array`

Since the induced sorting method is O(n), it's a lot faster than the usual O(nlogn) sorting, and it's also memory efficient.

    bench_sort_rotations_1000_random_values         ... bench:     292,912 ns/iter (+/- 24,688)
    bench_suffix_array_1000_random_values           ... bench:     100,227 ns/iter (+/- 16,021)

#### `FMIndex`

FM-index is very fast in its construction and getting, but it consumes a lot of memory (almost the same as the suffix array). There are multiple ways to optimize this (I'll try to do it in the future).

    bench_fm_index_1000_random_values_constructor   ... bench:     115,514 ns/iter (+/- 20,053)
    bench_fm_index_1000_random_values_get_100_chars ... bench:       1,094 ns/iter (+/- 78)


