#![doc(html_logo_url = "https://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "https://www.rust-lang.org/favicon.ico", html_root_url = ".")]
extern crate bit_vec;
extern crate num_traits;

mod bits_vec;
mod bwt;
mod sa;

pub use bwt::{bwt, ibwt, FMIndex};
pub use bits_vec::{BitsVec, ReprUsize};
pub use sa::suffix_array;
