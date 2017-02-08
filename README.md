## rust-helix

This Rust library has some of the bioinformatics stuff I'd written for playing with DNA sequences. It has the following implementations:

 - **Trie** - for getting/inserting a value corresponding to an iterator of elements.
 - **BWT** - for generating the Burrows-Wheeler Transform (for the given text) using a suffix array (constructed by the induced sorting method with O(n) space in O(n) time).
 - **FM-Index** - It uses the BWT to count/get the occurrences of substrings in O(1) time. This is the backbone of sequence alignment (Note that it's unoptimized in terms of memory).
 - **Bits Vector** - DNA sequences are almost always a bunch of ATCGs. Using 2 bits to represent a nucleotide instead of the usual byte (8 bits) could save *a lot* of memory! `BitsVec` provides a generic interface for stuff that have a known bit range.
