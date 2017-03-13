#![doc(html_logo_url = "https://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "https://www.rust-lang.org/favicon.ico", html_root_url = ".")]
extern crate bit_vec;
extern crate fillings;
extern crate num_traits;
extern crate rustc_serialize;

mod bwt;
mod sa;

pub use bwt::{bwt, ibwt, FMIndex};
pub use fillings::{BitsVec, ReprUsize};
pub use sa::suffix_array;
