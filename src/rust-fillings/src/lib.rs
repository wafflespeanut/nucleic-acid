use std::fmt;
use std::marker::PhantomData;
use std::ops::Range;
use std::usize;

pub trait ReprUsize {
    fn from_usize(usize) -> Self;
    fn into_usize(self) -> usize;
}

impl ReprUsize for bool {
    fn from_usize(i: usize) -> bool {
        match i {
            0 => false,
            1 => true,
            _ => unreachable!(),
        }
    }

    fn into_usize(self) -> usize { self as usize }
}

impl ReprUsize for char {
    fn from_usize(i: usize) -> char { i as u8 as char }
    fn into_usize(self) -> usize { self as u8 as usize }
}

macro_rules! impl_predefined_type {
    ($ty: ty) => {
        impl ReprUsize for $ty {
            fn from_usize(i: usize) -> $ty { i as $ty }
            fn into_usize(self) -> usize { self as usize }
        }
    };
}

impl_predefined_type!(u8);
impl_predefined_type!(u16);
impl_predefined_type!(u32);
impl_predefined_type!(u64);
impl_predefined_type!(usize);
impl_predefined_type!(i8);
impl_predefined_type!(i16);
impl_predefined_type!(i32);
impl_predefined_type!(i64);
impl_predefined_type!(isize);
impl_predefined_type!(f32);
impl_predefined_type!(f64);

#[derive(Clone, Hash)]
pub struct BitsVec<T: ReprUsize> {
    inner: Vec<usize>,
    units: usize,
    bits: usize,
    max_bits: usize,
    leftover: usize,
    _marker: PhantomData<T>,
}

impl<T: ReprUsize> BitsVec<T> {
    pub fn new(bits: usize) -> BitsVec<T> {
        let max = usize::MAX.count_ones() as usize;
        // We can store more bits, but then we might need BigInt to get them out!
        assert!(bits < max, "cannot hold more than {} bits at a time", max - 1);

        BitsVec {
            inner: vec![0],
            units: 0,
            bits: bits,
            max_bits: max,
            leftover: max,
            _marker: PhantomData,
        }
    }

    pub fn push(&mut self, value: T) {
        let mut value = value.into_usize();
        assert!(value >> self.bits == 0,
                "input size is more than allowed size ({} >= {})", value, 2usize.pow(self.bits as u32));

        let mut idx = self.inner.len() - 1;
        let shift;

        if self.leftover < self.bits {
            let left = self.bits - self.leftover;
            self.inner[idx] |= value >> left;
            if self.leftover != 0 {     // special case, in which masking would result in zero!
                value &= (1 << left) - 1;
            }

            self.inner.push(0);
            self.leftover = self.max_bits - left;
            shift = self.max_bits - left;
            idx += 1;
        } else {
            shift = self.leftover - self.bits;
            self.leftover -= self.bits;
        }

        value <<= shift;
        self.inner[idx] |= value;
        self.units += 1;
    }

    pub fn get(&self, i: usize) -> Option<T> {
        if i >= self.units {
            return None
        }

        let idx = i * self.bits / self.max_bits;
        let bits = (i * self.bits) % self.max_bits;
        let diff = self.max_bits - bits;
        let mut val = self.inner[idx];
        if bits != 0 {
            val &= (1 << diff) - 1;
        }

        if diff >= self.bits {
            Some(T::from_usize(val >> (diff - self.bits)))
        } else {
            let shift = self.bits - diff;
            let out = (val << shift) | (self.inner[idx + 1] >> (self.max_bits - shift));
            Some(T::from_usize(out))
        }
    }

    pub fn set(&mut self, i: usize, value: T) {
        let value = value.into_usize();
        assert!(i < self.units, "index out of bounds ({} >= {})", i, self.units);
        assert!(value >> self.bits == 0,
                "input size is more than allowed size ({} >= {})", value, 2usize.pow(self.bits as u32));

        let idx = i * self.bits / self.max_bits;
        let bits = (i * self.bits) % self.max_bits;
        let diff = self.max_bits - bits;
        let mut val = self.inner[idx];

        if diff >= self.bits {
            let shift = diff - self.bits;
            let last = val & ((1 << shift) - 1);
            let mask = if bits == 0 { 0 } else { ((1 << bits) - 1) << diff };   // prevent overflow
            val &= mask;
            val |= value << shift;
            self.inner[idx] = val | last;
        } else {
            let shift = self.bits - diff;
            self.inner[idx] >>= diff;
            self.inner[idx] <<= diff;
            self.inner[idx] |= value >> shift;
            let last = value & ((1 << shift) - 1);
            self.inner[idx + 1] &= (1 << (self.max_bits - shift)) - 1;
            self.inner[idx + 1] |= last << (self.max_bits - shift);
        }
    }

    pub fn len(&self) -> usize {
        self.units
    }

    pub fn inner_len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter(&self) -> Iter<T> {
        Iter { vec: self, range: 0..self.units }
    }

    pub fn into_iter(self) -> IntoIter<T> {
        IntoIter { range: 0..self.units, vec: self }
    }
}

impl<T: ReprUsize + fmt::Debug> fmt::Debug for BitsVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

pub struct Iter<'a, T: ReprUsize + 'a> {
    vec: &'a BitsVec<T>,
    range: Range<usize>,
}

impl<'a, T: ReprUsize> IntoIterator for &'a BitsVec<T> {
    type Item = T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T: ReprUsize> Iterator for Iter<'a, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.range.next().and_then(|i| self.vec.get(i))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl<'a, T: ReprUsize> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<T> {
        self.range.next_back().and_then(|i| self.vec.get(i))
    }
}

impl<'a, T: ReprUsize> ExactSizeIterator for Iter<'a, T> {}

pub struct IntoIter<T: ReprUsize> {
    vec: BitsVec<T>,
    range: Range<usize>,
}

impl<T: ReprUsize> IntoIterator for BitsVec<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> IntoIter<T> {
        self.into_iter()
    }
}

impl<T: ReprUsize> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.range.next().and_then(|i| self.vec.get(i))
    }
}

impl<T: ReprUsize> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.range.next_back().and_then(|i| self.vec.get(i))
    }
}

impl<T: ReprUsize> ExactSizeIterator for IntoIter<T> {}
