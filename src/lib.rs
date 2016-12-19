extern crate fillings;

mod bwt;
mod sa;
mod trie;

pub use bwt::{bwt, ibwt, FMIndex};
pub use fillings::BitsVec;
pub use sa::suffix_array;
pub use trie::Trie;
