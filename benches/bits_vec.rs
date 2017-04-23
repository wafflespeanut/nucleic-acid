#![feature(test)]
extern crate test;
extern crate nucleic_acid;

use nucleic_acid::BitsVec;
use test::Bencher;

#[bench]
fn bench_1_bits_vec_fill_with_1000_elements(b: &mut Bencher) {
    b.iter(|| {
        BitsVec::with_elements(1, 1000, 1);
    });
}

#[bench]
fn bench_1_bits_vec_push_1000_ints(b: &mut Bencher) {
    let mut vec = BitsVec::new(1);
    b.iter(|| {
        for _ in 0..1000 {
            vec.push(1);
        }
    });
}

#[bench]
fn bench_1_bits_vec_get_1000_ints(b: &mut Bencher) {
    let vec = BitsVec::with_elements(1, 1000, 1);
    b.iter(|| {
        for i in 0..1000 {
            vec.get(i);
        }
    });
}

#[bench]
fn bench_1_bits_vec_set_1000_ints(b: &mut Bencher) {
    let mut vec = BitsVec::with_elements(1, 1000, 0);
    b.iter(|| {
        for i in 0..1000 {
            vec.set(i, 1);
        }
    });
}

#[bench]
fn bench_22_bits_vec_fill_with_1000_elements(b: &mut Bencher) {
    b.iter(|| {
        BitsVec::with_elements(22, 1000, 100);
    });
}

#[bench]
fn bench_22_bits_vec_push_1000_ints(b: &mut Bencher) {
    let mut vec = BitsVec::new(22);
    b.iter(|| {
        for _ in 0..1000 {
            vec.push(100);
        }
    });
}

#[bench]
fn bench_22_bits_vec_get_1000_ints(b: &mut Bencher) {
    let vec = BitsVec::with_elements(22, 1000, 100);
    b.iter(|| {
        for i in 0..1000 {
            vec.get(i);
        }
    });
}

#[bench]
fn bench_22_bits_vec_set_1000_ints(b: &mut Bencher) {
    let mut vec = BitsVec::with_elements(22, 1000, 100);
    b.iter(|| {
        for i in 0..1000 {
            vec.set(i, 99);
        }
    });
}

#[bench]
fn bench_40_bits_vec_fill_with_1000_elements(b: &mut Bencher) {
    b.iter(|| {
        BitsVec::with_elements(40, 1000, 100);
    });
}

#[bench]
fn bench_40_bits_vec_push_1000_ints(b: &mut Bencher) {
    let mut vec = BitsVec::new(40);
    b.iter(|| {
        for _ in 0..1000 {
            vec.push(100);
        }
    });
}

#[bench]
fn bench_40_bits_vec_get_1000_ints(b: &mut Bencher) {
    let vec = BitsVec::with_elements(40, 1000, 100);
    b.iter(|| {
        for i in 0..1000 {
            vec.get(i);
        }
    });
}

#[bench]
fn bench_40_bits_vec_set_1000_ints(b: &mut Bencher) {
    let mut vec = BitsVec::with_elements(40, 1000, 100);
    b.iter(|| {
        for i in 0..1000 {
            vec.set(i, 99);
        }
    });
}

#[bench]
fn bench_63_bits_vec_fill_with_1000_elements(b: &mut Bencher) {
    b.iter(|| {
        BitsVec::with_elements(63, 1000, 100);
    });
}

#[bench]
fn bench_63_bits_vec_push_1000_ints(b: &mut Bencher) {
    let mut vec = BitsVec::new(63);
    b.iter(|| {
        for _ in 0..1000 {
            vec.push(100);
        }
    });
}

#[bench]
fn bench_63_bits_vec_get_1000_ints(b: &mut Bencher) {
    let vec = BitsVec::with_elements(63, 1000, 100);
    b.iter(|| {
        for i in 0..1000 {
            vec.get(i);
        }
    });
}

#[bench]
fn bench_63_bits_vec_set_1000_ints(b: &mut Bencher) {
    let mut vec = BitsVec::with_elements(63, 1000, 100);
    b.iter(|| {
        for i in 0..1000 {
            vec.set(i, 99);
        }
    });
}
