//! Heap-allocated big unsigned integers.

mod add;
mod add_mod;
mod bit_and;
mod bit_or;
mod bits;
mod cmp;
mod ct;
mod div;
pub(crate) mod encoding;
mod inv_mod;
mod mul;
mod mul_mod;
mod neg;
mod shl;
mod shr;
mod sub;
mod sub_mod;

mod bit_not;
mod bit_xor;
mod neg_mod;
#[cfg(feature = "rand_core")]
mod rand;

use crate::{Integer, Limb, NonZero, Uint, Word, Zero, U128, U64};
use alloc::{boxed::Box, vec, vec::Vec};
use core::{fmt, mem};
use subtle::{Choice, ConstantTimeEq};

#[cfg(feature = "zeroize")]
use zeroize::Zeroize;

/// Fixed-precision heap-allocated big unsigned integer.
///
/// Alternative to the stack-allocated [`Uint`][`crate::Uint`] but with a
/// fixed precision chosen at runtime instead of compile time.
///
/// Unlike many other heap-allocated big integer libraries, this type is not
/// arbitrary precision and will wrap at its fixed-precision rather than
/// automatically growing.
#[allow(clippy::derived_hash_with_manual_eq)]
#[derive(Clone, Hash)]
pub struct BoxedUint {
    /// Boxed slice containing limbs.
    ///
    /// Stored from least significant to most significant.
    pub(crate) limbs: Box<[Limb]>,
}

impl BoxedUint {
    fn limbs_for_precision(at_least_bits_precision: u32) -> usize {
        ((at_least_bits_precision + Limb::BITS - 1) / Limb::BITS) as usize
    }

    /// Get the value `0` represented as succinctly as possible.
    pub fn zero() -> Self {
        Self {
            limbs: vec![Limb::ZERO; 1].into(),
        }
    }

    /// Get the value `0` with the given number of bits of precision.
    ///
    /// `at_least_bits_precision` is rounded up to a multiple of [`Limb::BITS`].
    pub fn zero_with_precision(at_least_bits_precision: u32) -> Self {
        vec![Limb::ZERO; Self::limbs_for_precision(at_least_bits_precision)].into()
    }

    /// Get the value `1`, represented as succinctly as possible.
    pub fn one() -> Self {
        Self {
            limbs: vec![Limb::ONE; 1].into(),
        }
    }

    /// Get the value `1` with the given number of bits of precision.
    ///
    /// `at_least_bits_precision` is rounded up to a multiple of [`Limb::BITS`].
    pub fn one_with_precision(at_least_bits_precision: u32) -> Self {
        let mut ret = Self::zero_with_precision(at_least_bits_precision);
        ret.limbs[0] = Limb::ONE;
        ret
    }

    /// Is this [`BoxedUint`] equal to zero?
    pub fn is_zero(&self) -> Choice {
        self.limbs
            .iter()
            .fold(Choice::from(1), |acc, limb| acc & limb.is_zero())
    }

    /// Is this [`BoxedUint`] equal to one?
    pub fn is_one(&self) -> Choice {
        let mut iter = self.limbs.iter();
        let choice = iter.next().copied().unwrap_or(Limb::ZERO).ct_eq(&Limb::ONE);
        iter.fold(choice, |acc, limb| acc & limb.is_zero())
    }

    /// Is this integer value an odd number?
    ///
    /// # Returns
    ///
    /// If odd, returns `Choice(1)`. Otherwise, returns `Choice(0)`.
    // TODO(tarcieri): impl the `Integer` trait
    pub fn is_odd(&self) -> Choice {
        self.limbs
            .first()
            .map(|limb| limb.is_odd())
            .unwrap_or_else(|| Choice::from(0))
    }

    /// Is this integer value an even number?
    ///
    /// # Returns
    ///
    /// If even, returns `Choice(1)`. Otherwise, returns `Choice(0)`.
    // TODO(tarcieri): impl the `Integer` trait
    pub fn is_even(&self) -> Choice {
        !self.is_odd()
    }

    /// Get the maximum value for a `BoxedUint` created with `at_least_bits_precision`
    /// precision bits requested.
    ///
    /// That is, returns the value `2^self.bits_precision() - 1`.
    pub fn max(at_least_bits_precision: u32) -> Self {
        vec![Limb::MAX; Self::limbs_for_precision(at_least_bits_precision)].into()
    }

    /// Create a [`BoxedUint`] from an array of [`Word`]s (i.e. word-sized unsigned
    /// integers).
    #[inline]
    pub fn from_words(words: impl IntoIterator<Item = Word>) -> Self {
        Self {
            limbs: words.into_iter().map(Into::into).collect(),
        }
    }

    /// Create a boxed slice of [`Word`]s (i.e. word-sized unsigned integers) from
    /// a [`BoxedUint`].
    #[inline]
    pub fn to_words(&self) -> Box<[Word]> {
        self.limbs.iter().copied().map(Into::into).collect()
    }

    /// Borrow the inner limbs as a slice of [`Word`]s.
    pub fn as_words(&self) -> &[Word] {
        // SAFETY: `Limb` is a `repr(transparent)` newtype for `Word`
        #[allow(trivial_casts, unsafe_code)]
        unsafe {
            &*((&*self.limbs as *const _) as *const [Word])
        }
    }

    /// Borrow the inner limbs as a mutable slice of [`Word`]s.
    pub fn as_words_mut(&mut self) -> &mut [Word] {
        // SAFETY: `Limb` is a `repr(transparent)` newtype for `Word`
        #[allow(trivial_casts, unsafe_code)]
        unsafe {
            &mut *((&mut *self.limbs as *mut _) as *mut [Word])
        }
    }

    /// Borrow the limbs of this [`BoxedUint`].
    pub fn as_limbs(&self) -> &[Limb] {
        self.limbs.as_ref()
    }

    /// Borrow the limbs of this [`BoxedUint`] mutably.
    pub fn as_limbs_mut(&mut self) -> &mut [Limb] {
        self.limbs.as_mut()
    }

    /// Convert this [`BoxedUint`] into its inner limbs.
    pub fn to_limbs(&self) -> Box<[Limb]> {
        self.limbs.clone()
    }

    /// Convert this [`BoxedUint`] into its inner limbs.
    pub fn into_limbs(self) -> Box<[Limb]> {
        self.limbs
    }

    /// Get the number of limbs in this [`BoxedUint`].
    pub fn nlimbs(&self) -> usize {
        self.limbs.len()
    }

    /// Widen this type's precision to the given number of bits.
    ///
    /// Panics if `at_least_bits_precision` is smaller than the current precision.
    pub fn widen(&self, at_least_bits_precision: u32) -> BoxedUint {
        assert!(at_least_bits_precision >= self.bits_precision());

        let mut ret = BoxedUint::zero_with_precision(at_least_bits_precision);
        ret.limbs[..self.nlimbs()].copy_from_slice(&self.limbs);
        ret
    }

    /// Shortens this type's precision to the given number of bits.
    ///
    /// Panics if `at_least_bits_precision` is larger than the current precision.
    pub fn shorten(&self, at_least_bits_precision: u32) -> BoxedUint {
        assert!(at_least_bits_precision <= self.bits_precision());
        let mut ret = BoxedUint::zero_with_precision(at_least_bits_precision);
        let nlimbs = ret.nlimbs();
        ret.limbs.copy_from_slice(&self.limbs[..nlimbs]);
        ret
    }

    /// Perform a carry chain-like operation over the limbs of the inputs,
    /// constructing a result from the returned limbs and carry which is
    /// widened to the same width as the widest input.
    ///
    /// If one of the two values has fewer limbs than the other, pads with
    /// [`Limb::ZERO`] as the value for that limb.
    #[inline]
    fn fold_limbs<F>(lhs: &Self, rhs: &Self, mut carry: Limb, f: F) -> (Self, Limb)
    where
        F: Fn(Limb, Limb, Limb) -> (Limb, Limb),
    {
        let nlimbs = cmp::max(lhs.nlimbs(), rhs.nlimbs());
        let mut limbs = Vec::with_capacity(nlimbs);

        for i in 0..nlimbs {
            let &a = lhs.limbs.get(i).unwrap_or(&Limb::ZERO);
            let &b = rhs.limbs.get(i).unwrap_or(&Limb::ZERO);
            let (limb, c) = f(a, b, carry);
            limbs.push(limb);
            carry = c;
        }

        (limbs.into(), carry)
    }

    /// Iterate over the limbs of the inputs, applying the given function, and
    /// constructing a result from the returned values.
    #[inline]
    fn map_limbs<F>(lhs: &Self, rhs: &Self, f: F) -> Self
    where
        F: Fn(Limb, Limb) -> Limb,
    {
        let nlimbs = cmp::max(lhs.nlimbs(), rhs.nlimbs());
        let mut limbs = Vec::with_capacity(nlimbs);

        for i in 0..nlimbs {
            let &a = lhs.limbs.get(i).unwrap_or(&Limb::ZERO);
            let &b = rhs.limbs.get(i).unwrap_or(&Limb::ZERO);
            limbs.push(f(a, b));
        }

        limbs.into()
    }
}

impl NonZero<BoxedUint> {
    /// Widen this type's precision to the given number of bits.
    ///
    /// See [`BoxedUint::widen`] for more information, including panic conditions.
    pub fn widen(&self, bits_precision: u32) -> Self {
        NonZero(self.0.widen(bits_precision))
    }
}

impl AsRef<[Word]> for BoxedUint {
    fn as_ref(&self) -> &[Word] {
        self.as_words()
    }
}

impl AsMut<[Word]> for BoxedUint {
    fn as_mut(&mut self) -> &mut [Word] {
        self.as_words_mut()
    }
}

impl AsRef<[Limb]> for BoxedUint {
    fn as_ref(&self) -> &[Limb] {
        self.as_limbs()
    }
}

impl AsMut<[Limb]> for BoxedUint {
    fn as_mut(&mut self) -> &mut [Limb] {
        self.as_limbs_mut()
    }
}

impl Default for BoxedUint {
    fn default() -> Self {
        Self::zero()
    }
}

impl From<u8> for BoxedUint {
    fn from(n: u8) -> Self {
        vec![Limb::from(n); 1].into()
    }
}

impl From<u16> for BoxedUint {
    fn from(n: u16) -> Self {
        vec![Limb::from(n); 1].into()
    }
}

impl From<u32> for BoxedUint {
    fn from(n: u32) -> Self {
        vec![Limb::from(n); 1].into()
    }
}

impl From<u64> for BoxedUint {
    fn from(n: u64) -> Self {
        U64::from(n).into()
    }
}

impl From<u128> for BoxedUint {
    fn from(n: u128) -> Self {
        U128::from(n).into()
    }
}

impl From<Limb> for BoxedUint {
    fn from(limb: Limb) -> Self {
        vec![limb; 1].into()
    }
}

impl From<&[Limb]> for BoxedUint {
    fn from(limbs: &[Limb]) -> BoxedUint {
        Self {
            limbs: limbs.into(),
        }
    }
}

impl From<Box<[Limb]>> for BoxedUint {
    fn from(limbs: Box<[Limb]>) -> BoxedUint {
        Vec::from(limbs).into()
    }
}

impl From<Vec<Limb>> for BoxedUint {
    fn from(mut limbs: Vec<Limb>) -> BoxedUint {
        if limbs.is_empty() {
            limbs.push(Limb::ZERO);
        }

        Self {
            limbs: limbs.into_boxed_slice(),
        }
    }
}

impl From<Vec<Word>> for BoxedUint {
    fn from(mut words: Vec<Word>) -> BoxedUint {
        // SAFETY: `Limb` is a `repr(transparent)` newtype for `Word`
        #[allow(unsafe_code)]
        unsafe {
            let ptr = words.as_mut_ptr() as *mut Limb;
            let len = words.len();
            let capacity = words.capacity();
            mem::forget(words);
            Vec::<Limb>::from_raw_parts(ptr, len, capacity)
        }
        .into()
    }
}

impl<const LIMBS: usize> From<Uint<LIMBS>> for BoxedUint {
    fn from(uint: Uint<LIMBS>) -> BoxedUint {
        Vec::from(uint.to_limbs()).into()
    }
}

impl Integer for BoxedUint {
    fn one() -> Self {
        Self::one()
    }

    fn bits(&self) -> u32 {
        self.bits()
    }

    fn bits_vartime(&self) -> u32 {
        self.bits_vartime()
    }

    fn bits_precision(&self) -> u32 {
        self.bits_precision()
    }

    fn bytes_precision(&self) -> usize {
        self.nlimbs() * Limb::BYTES
    }

    fn nlimbs(&self) -> usize {
        self.nlimbs()
    }

    fn is_odd(&self) -> Choice {
        self.is_odd()
    }
}

impl Zero for BoxedUint {
    fn zero() -> Self {
        Self::zero()
    }

    fn is_zero(&self) -> Choice {
        self.is_zero()
    }
}

#[cfg(feature = "zeroize")]
impl Zeroize for BoxedUint {
    fn zeroize(&mut self) {
        self.limbs.zeroize();
    }
}

impl fmt::Debug for BoxedUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BoxedUint(0x{self:X})")
    }
}

impl fmt::Display for BoxedUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(self, f)
    }
}

impl fmt::LowerHex for BoxedUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.limbs.is_empty() {
            return fmt::LowerHex::fmt(&Limb::ZERO, f);
        }

        for limb in self.limbs.iter().rev() {
            fmt::LowerHex::fmt(limb, f)?;
        }
        Ok(())
    }
}

impl fmt::UpperHex for BoxedUint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.limbs.is_empty() {
            return fmt::LowerHex::fmt(&Limb::ZERO, f);
        }

        for limb in self.limbs.iter().rev() {
            fmt::UpperHex::fmt(limb, f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::BoxedUint;
    use crate::Word;
    use alloc::vec::Vec;

    #[test]
    fn from_word_vec() {
        let words: &[Word] = &[0, 1, 2, 3];
        let uint = BoxedUint::from(Vec::from(words));
        assert_eq!(uint.nlimbs(), 4);
        assert_eq!(uint.as_words(), words);
    }
}
