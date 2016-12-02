use std::fmt;
use std::ops::Range;
use std::usize;

#[derive(Clone, Hash)]
pub struct BitsVec {
    inner: Vec<usize>,
    units: usize,
    bits: usize,
    max_bits: usize,
    leftover: usize,
}

impl BitsVec {
    pub fn new(bits: usize) -> BitsVec {
        let max = usize::MAX.count_ones() as usize;
        // We can store more bits, but then we might need BigInt to get them out!
        assert!(bits < max, "cannot hold more than {} bits at a time", max - 1);

        BitsVec {
            inner: vec![0],
            units: 0,
            bits: bits,
            max_bits: max,
            leftover: max,
        }
    }

    pub fn push(&mut self, mut value: usize) {
        assert!(value >> self.bits == 0,
                "input size is more than allowed size ({} >= {})", value, 2usize.pow(self.bits as u32));

        let mut idx = self.inner.len() - 1;
        let shift_amount;

        if self.leftover < self.bits {
            let left = self.bits - self.leftover;
            self.inner[idx] |= value >> left;
            if self.leftover != 0 {     // special case, in which masking would result in zero!
                value &= (1 << left) - 1;
            }

            self.inner.push(0);
            self.leftover = self.max_bits - left;
            shift_amount = self.max_bits - left;
            idx += 1;
        } else {
            shift_amount = self.leftover - self.bits;
            self.leftover -= self.bits;
        }

        value <<= shift_amount;
        self.inner[idx] |= value;
        self.units += 1;
    }

    pub fn get(&self, i: usize) -> Option<usize> {
        if i >= self.units {
            return None
        }

        let idx = i * self.bits / self.max_bits;
        let bits = (i * self.bits) % self.max_bits;
        let mut val = self.inner[idx];
        if bits != 0 {
            val &= (1 << (self.max_bits - bits)) - 1;
        }

        let diff = self.max_bits - bits;
        if diff >= self.bits {
            Some(val >> (diff - self.bits))
        } else {
            let shift = self.bits - diff;
            Some((val << shift) | (self.inner[idx + 1] >> (self.max_bits - shift)))
        }
    }

    pub fn len(&self) -> usize {
        self.units
    }

    pub fn inner_len(&self) -> usize {
        self.inner.len()
    }

    pub fn iter(&self) -> Iter {
        Iter { vec: self, range: 0..self.units }
    }

    pub fn into_iter(self) -> IntoIter {
        IntoIter { range: 0..self.units, vec: self }
    }
}

impl fmt::Debug for BitsVec {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

pub struct Iter<'a> {
    vec: &'a BitsVec,
    range: Range<usize>,
}

impl<'a> IntoIterator for &'a BitsVec {
    type Item = usize;
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        self.range.next().and_then(|i| self.vec.get(i))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<usize> {
        self.range.next_back().and_then(|i| self.vec.get(i))
    }
}

impl<'a> ExactSizeIterator for Iter<'a> {}

pub struct IntoIter {
    vec: BitsVec,
    range: Range<usize>,
}

impl IntoIterator for BitsVec {
    type Item = usize;
    type IntoIter = IntoIter;

    fn into_iter(self) -> IntoIter {
        self.into_iter()
    }
}

impl Iterator for IntoIter {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        self.range.next().and_then(|i| self.vec.get(i))
    }
}

impl DoubleEndedIterator for IntoIter {
    fn next_back(&mut self) -> Option<usize> {
        self.range.next_back().and_then(|i| self.vec.get(i))
    }
}

impl ExactSizeIterator for IntoIter {}
