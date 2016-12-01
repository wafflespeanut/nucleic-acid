use std::usize;

#[derive(Eq, PartialEq, Clone, Hash)]
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
        if bits > max {
            // We can store more bits, but then we might need BigInt to get them out!
            panic!("cannot hold more than {} bits at a time", max);
        }

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
}
