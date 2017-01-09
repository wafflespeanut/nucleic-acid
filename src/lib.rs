extern crate bincode;
extern crate bit_vec;
extern crate fillings;
extern crate num_traits;
extern crate rand;
extern crate rustc_serialize;

mod bwt;
mod sa;
mod trie;

pub use bwt::{bwt, ibwt, FMIndex};
pub use fillings::{BitsVec, ReprUsize};
pub use sa::suffix_array;
pub use trie::Trie;
