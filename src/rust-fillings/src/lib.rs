extern crate rustc_serialize;

use std::cmp;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::ops::Range;
use std::usize;

/// A trait for representing types as `usize` (useful for enums).
///
/// Note that this should be implemented for types that need to be passed to `BitsVec`.
/// This is implemented for integer types, `bool` and `char` by default.
///
/// ``` rust
/// enum Foo {
///     One,
///     Two,
/// }
///
/// impl ReprUsize for Foo {
///     fn into_usize(self) -> usize {
///         match self {
///             Foo::One => 0,
///             Foo::Two => 1,
///         }
///     }
///
///    fn from_usze(i: usize) -> Foo {
///         match i {
///             0 => Foo::One,
///             1 => Foo::Two,
///             _ => unimplemented!(),
///         }
///     }
/// }
/// ```
///
pub trait ReprUsize {
    /// Convert the value back from `usize`
    fn from_usize(usize) -> Self;
    /// Convert the value into an `usize`
    fn into_usize(self) -> usize;
}

impl ReprUsize for bool {
    fn into_usize(self) -> usize { self as usize }
    fn from_usize(i: usize) -> bool {
        unsafe { mem::transmute(i as u8) }
    }
}

impl ReprUsize for char {
    fn into_usize(self) -> usize { self as usize }
    fn from_usize(i: usize) -> char {
        unsafe { mem::transmute(i as u32) }
    }
}

macro_rules! impl_predefined_type {
    ($ty: ty) => {
        impl ReprUsize for $ty {
            fn into_usize(self) -> usize { self as usize }
            fn from_usize(i: usize) -> $ty { i as $ty }
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

#[derive(Clone, Hash, RustcEncodable, RustcDecodable)]
/// A vector to hold values that have a known bit range.
///
/// For example, DNA nucleotides don't need 8 bits to represent them. We know they
/// only have four possible values, so 2 bits would be enough.
///
/// ``` rust
/// extern crate helix;
///
/// use helix::{BitsVec, ReprUsize};
/// use std::mem;
///
/// #[derive(Clone, Copy, Debug)]
/// #[repr(usize)]
/// enum Nucleotide {
///     Adenine,
///     Thymine,
///     Guanine,
///     Cytosine,
/// }
///
/// impl ReprUsize for Nucleotide {
///     fn from_usize(i: usize) -> Self {
///         assert!(i <= 3, "expected vales in the range [0, 3]");
///         unsafe { mem::transmute(i) }
///     }
///
///     fn into_usize(self) -> usize {
///         self as usize
///     }
/// }
///
/// fn main() {
///     let vec = BitsVec::with_elements(2, 100, Nucleotide::Adenine);
///     assert!(vec.len() == 100);
///     // depends on the architecture (since BitsVec uses Vec<usize> inside)
///     assert!(vec.inner_len() == 2 || vec.inner_len() == 4);
/// }
///
/// ```
///
/// The human genome has ~3 billion bases (that's 3 GB). Using 8 bits for each of them would be
/// a waste of space. This representation reduces the memory consumed by a factor of 6.
///
pub struct BitsVec<T: ReprUsize> {
    inner: Vec<usize>,
    units: usize,
    bits: usize,
    max_bits: usize,
    leftover: usize,
    _marker: PhantomData<T>,
}

impl<T: ReprUsize> BitsVec<T> {
    /// Create a new vector that can hold values no larger than the specified `bits`
    pub fn new(bits: usize) -> BitsVec<T> {
        let max = usize::MAX.count_ones() as usize;
        // We can store more bits, but then we might need BigInt to get them out!
        assert!(bits < max, "[new] cannot hold more than {} bits at a time", max - 1);

        BitsVec {
            inner: vec![0],
            units: 0,
            bits: bits,
            max_bits: max,
            leftover: max,
            _marker: PhantomData,
        }
    }

    /// Creates a new vector that can hold the specified bits (atmost) and has capacity
    /// for "N" additional elements.
    pub fn with_capacity(bits: usize, capacity: usize) -> BitsVec<T> {
        let mut vec = BitsVec::new(bits);
        vec.reserve(capacity);
        vec
    }

    /// Push a value into the vector.
    pub fn push(&mut self, value: T) {
        let mut value = value.into_usize();
        assert!(value >> self.bits == 0,
                "[push] input size is more than allowed size ({} >= {})", value, 2usize.pow(self.bits as u32));

        let mut idx = self.inner.len() - 1;
        if self.leftover < self.bits {
            let left = self.bits - self.leftover;
            self.inner[idx] |= value >> left;
            if self.leftover != 0 {     // special case, in which masking would result in zero!
                value &= (1 << left) - 1;
            }

            self.inner.push(0);
            self.leftover = self.max_bits - left;
            idx += 1;
        } else {
            self.leftover -= self.bits;
        }

        value <<= self.leftover;
        self.inner[idx] |= value;
        self.units += 1;
    }

    /// Get the value from an index in the vector. Note that this is similar to indexed getting,
    /// and so it panics when the index is out of bounds. For the non-panicking version, use `checked_get`
    pub fn get(&self, i: usize) -> T {
        assert!(i < self.units, "[get] index out of bounds ({} >= {})", i, self.units);

        let pos = i * self.bits;
        let idx = pos / self.max_bits;
        let bits = pos % self.max_bits;
        let diff = self.max_bits - bits;
        let mut val = self.inner[idx];
        if bits != 0 {
            val &= (1 << diff) - 1;
        }

        if diff >= self.bits {
            T::from_usize(val >> (diff - self.bits))
        } else {
            let shift = self.bits - diff;
            let last = self.inner[idx + 1] >> (self.max_bits - shift);
            T::from_usize((val << shift) | last)
        }
    }

    /// Returns `Some(T)` if the element exists at the given index or `None` if it doesn't.
    pub fn checked_get(&self, i: usize) -> Option<T> {
        if i >= self.units {
            return None
        }

        Some(self.get(i))
    }

    /// Set a value at the given index. Note that this is similar to indexed setting, and so it
    /// panics when the index is out of bounds.
    pub fn set(&mut self, i: usize, value: T) {
        let value = value.into_usize();
        assert!(i < self.units, "[set] index out of bounds ({} >= {})", i, self.units);
        assert!(value >> self.bits == 0,
                "[set] input size is more than allowed size ({} >= {})", value, 2usize.pow(self.bits as u32));

        let pos = i * self.bits;
        let idx = pos / self.max_bits;
        let bits = pos % self.max_bits;
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
            val &= !((1 << diff) - 1);
            self.inner[idx] = val | (value >> shift);
            let last = value & ((1 << shift) - 1);
            let shift = self.max_bits - shift;
            self.inner[idx + 1] &= (1 << shift) - 1;
            self.inner[idx + 1] |= last << shift;
        }
    }

    /// Creates a vector consuming an iterator of elements.
    pub fn from_iter<I>(bits: usize, iterable: I) -> BitsVec<T>
        where I: Iterator<Item=T>
    {
        let mut vec = BitsVec::new(bits);
        for i in iterable {
            vec.push(i);
        }

        vec
    }

    /// Returns the length of the vector. This only indicates the number of units it contains,
    /// and not the length of the inner vector.
    pub fn len(&self) -> usize {
        self.units
    }

    /// Returns `true` if the vector contains no values (or `false` otherwise).
    pub fn is_empty(&self) -> bool {
        self.units == 0
    }

    /// Reserve space for "N" additional elements.
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional * self.bits / self.max_bits + 1);
    }

    /// Shrink the inner vector's capacity to fit to its length. It does nothing more than
    /// calling the same method in the inner vector.
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }

    /// Truncate the vector to the given length, removing the out-of-bound elements. Note that this
    /// method panics when the length is greater than current length.
    pub fn truncate(&mut self, length: usize) {
        assert!(length < self.units, "length should be smaller for truncation ({} >= {})", length, self.units);
        self.units = length;

        let bits = length * self.bits;
        let mut new_len = bits / self.max_bits;
        let used = bits % self.max_bits;
        if used > 0 {
            new_len += 1;
        }

        self.inner.truncate(new_len);
        if used > 0 {
            self.leftover = self.max_bits - used;
            self.inner[new_len - 1] &= ((1 << used) - 1) << self.leftover;
        } else {
            self.inner.push(0);
            self.leftover = self.max_bits;
        }
    }

    /// Clears the inner vector. Note that this is similar to calling `truncate` with zero.
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    /// Returns the length of the inner vector. Useful for measuring the memory consumption
    /// of the elements.
    pub fn inner_len(&self) -> usize {
        self.inner.len()
    }

    /// Creates an iterator over the elements. Note that unlike other iterators, this gives the elements
    /// themselves, and not their references.
    pub fn iter(&self) -> Iter<T> {
        Iter { vec: self, range: 0..self.units }
    }

    /// Creates an iterator consuming the vector.
    pub fn into_iter(self) -> IntoIter<T> {
        IntoIter { range: 0..self.units, vec: self }
    }
}

impl<T: ReprUsize + Clone> BitsVec<T> {
    /// Creates a vector initialized with "N" copies of the given element.
    pub fn with_elements(bits: usize, length: usize, value: T) -> BitsVec<T> {
        let mut vec = BitsVec::new(bits);
        vec.extend_with_element(length, value);
        vec
    }

    /// Extends the vector to the specified length, filling additional values with the given element.
    /// Note that this method panics when the specified length is shorter than the initial length.
    pub fn extend_with_element(&mut self, length: usize, value: T) {
        assert!(length > self.len(), "[extend] final length should be greater than the initial length");
        // Three phases (somewhat inefficient, using safe code and all, but much better than `push`)
        let mut remain = length - self.len();
        self.reserve(remain);

        // 1. Slow push until we get to a common multiple of (self.bits, self.max_bits)
        while self.leftover > 0 {
            self.push(value.clone());
            remain -= 1;
            if remain == 0 {
                return
            }
        }

        // 2. Do the same to a new BitsVec
        let mut temp = BitsVec::new(self.bits);
        temp.reserve(cmp::min(remain, self.max_bits));
        temp.push(value.clone());
        while temp.leftover > 0 && remain > 0 {
            temp.push(value.clone());
        }

        if remain == 0 {
            self.units += temp.units;
            self.inner.extend(&temp.inner);
            return
        }

        // 3. Extend from the new BitsVec
        while remain >= temp.units {
            self.inner.extend(&temp.inner);
            self.units += temp.units;
            remain -= temp.units;
        }

        for _ in 0..remain {    // remaining valus, if any
            self.push(value.clone());
        }
    }
}

impl<T: ReprUsize + PartialEq> BitsVec<T> {
    /// Checks whether the vector contains the given element in O(n) time.
    pub fn contains(&self, element: &T) -> bool {
        self.iter().find(|ref i| i == &element).is_some()
    }
}

impl<T: ReprUsize + fmt::Debug> fmt::Debug for BitsVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: ReprUsize> PartialEq for BitsVec<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.units != other.units || self.bits != other.bits {
            return false
        }

        self.inner == other.inner
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
        self.range.next().map(|i| self.vec.get(i))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl<'a, T: ReprUsize> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<T> {
        self.range.next_back().map(|i| self.vec.get(i))
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
        self.range.next().map(|i| self.vec.get(i))
    }
}

impl<T: ReprUsize> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.range.next_back().map(|i| self.vec.get(i))
    }
}

impl<T: ReprUsize> ExactSizeIterator for IntoIter<T> {}

#[cfg(test)]
mod tests {
    use super::{BitsVec, ReprUsize};
    use std::mem;

    #[repr(usize)]
    #[derive(Clone, Debug, Eq, PartialEq)]
    enum TestEnum {
        Value1,
        Value2,
        Value3,
        Value4,
    }

    impl ReprUsize for TestEnum {
        fn into_usize(self) -> usize { self as usize }
        fn from_usize(i: usize) -> Self { unsafe { mem::transmute(i) } }
    }

    #[test]
    fn test_everything_with_enum() {
        let mut vec = BitsVec::with_elements(4, 16, TestEnum::Value4);
        vec.set(0, TestEnum::Value1);
        vec.set(1, TestEnum::Value2);
        vec.set(2, TestEnum::Value3);
        assert_eq!(vec.len(), 16);
        assert_eq!(vec.inner_len(), 1);
        assert_eq!(vec.get(0), TestEnum::Value1);
        assert_eq!(vec.get(1), TestEnum::Value2);
        assert_eq!(vec.get(2), TestEnum::Value3);
        vec.push(TestEnum::Value4);
        for i in 3..vec.len() {
            assert_eq!(vec.get(i), TestEnum::Value4);
        }
    }

    #[test]
    fn test_truncate() {
        let mut vec = BitsVec::with_elements(7, 50, 13);
        vec.truncate(10);
        assert_eq!(vec.inner_len(), 2);
        assert_eq!(vec.get(9), 13);
        vec.push(25);
        assert_eq!(vec.get(10), 25);
        let mut vec = BitsVec::with_elements(8, 20, 50);
        vec.truncate(8);
        assert_eq!(vec.inner_len(), 2);
        assert_eq!(vec.get(7), 50);
        vec.push(20);
        assert_eq!(vec.get(8), 20);
    }
}
