#![feature(test)]

extern crate helix;
#[macro_use]
extern crate lazy_static;
extern crate test;
extern crate rand;

use helix::{suffix_array, FMIndex};
use rand::Rng;
use test::Bencher;

lazy_static! {
    static ref DATA: Vec<u8> = {
        let mut rng = rand::thread_rng();
        let bases = vec![65, 67, 71, 84];
        (0..1000).map(|_| bases[rng.gen_range(0, bases.len())]).collect()
    };
}

#[bench]
fn bench_sort_rotations_1000_random_values(b: &mut Bencher) {
    b.iter(|| {
        let mut rotations = (0..DATA.len()).map(|i| &DATA[i..]).collect::<Vec<_>>();
        rotations.sort();
    })
}

#[bench]
fn bench_suffix_array_1000_random_values(b: &mut Bencher) {
    b.iter(|| {
        suffix_array(DATA.clone());
    })
}

#[bench]
fn bench_fm_index_1000_random_values_constructor(b: &mut Bencher) {
    b.iter(|| {
        FMIndex::new(DATA.clone());
    })
}

#[bench]
fn bench_fm_index_1000_random_values_getter(b: &mut Bencher) {
    let index = FMIndex::new(DATA.clone());
    b.iter(|| {
        index.search("AAA");
    })
}
