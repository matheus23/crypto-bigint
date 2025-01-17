//! [`Uint`] bitwise left shift operations.

use crate::{CtChoice, Limb, Uint, Word};
use core::ops::{Shl, ShlAssign};

impl<const LIMBS: usize> Uint<LIMBS> {
    /// Computes `self << shift`.
    /// Returns zero if `shift >= Self::BITS`.
    pub const fn shl(&self, shift: u32) -> Self {
        let overflow = CtChoice::from_u32_lt(shift, Self::BITS).not();
        let shift = shift % Self::BITS;
        let mut result = *self;
        let mut i = 0;
        while i < Self::LOG2_BITS + 1 {
            let bit = CtChoice::from_u32_lsb((shift >> i) & 1);
            result = Uint::ct_select(&result, &result.shl_vartime(1 << i), bit);
            i += 1;
        }

        Uint::ct_select(&result, &Self::ZERO, overflow)
    }

    /// Computes `self << shift`.
    ///
    /// NOTE: this operation is variable time with respect to `shift` *ONLY*.
    ///
    /// When used with a fixed `shift`, this function is constant-time with respect
    /// to `self`.
    #[inline(always)]
    pub const fn shl_vartime(&self, shift: u32) -> Self {
        let mut limbs = [Limb::ZERO; LIMBS];

        if shift >= Self::BITS {
            return Self { limbs };
        }

        let shift_num = (shift / Limb::BITS) as usize;
        let rem = shift % Limb::BITS;

        let mut i = LIMBS;
        while i > shift_num {
            i -= 1;
            limbs[i] = self.limbs[i - shift_num];
        }

        let (new_lower, _carry) = (Self { limbs }).shl_limb(rem);
        new_lower
    }

    /// Computes a left shift on a wide input as `(lo, hi)`.
    ///
    /// NOTE: this operation is variable time with respect to `shift` *ONLY*.
    ///
    /// When used with a fixed `shift`, this function is constant-time with respect
    /// to `self`.
    #[inline(always)]
    pub const fn shl_vartime_wide(lower_upper: (Self, Self), shift: u32) -> (Self, Self) {
        let (lower, mut upper) = lower_upper;
        let new_lower = lower.shl_vartime(shift);
        upper = upper.shl_vartime(shift);
        if shift >= Self::BITS {
            upper = upper.bitor(&lower.shl_vartime(shift - Self::BITS));
        } else {
            upper = upper.bitor(&lower.shr_vartime(Self::BITS - shift));
        }

        (new_lower, upper)
    }

    /// Computes `self << shift` where `0 <= shift < Limb::BITS`,
    /// returning the result and the carry.
    #[inline(always)]
    pub(crate) const fn shl_limb(&self, shift: u32) -> (Self, Limb) {
        let mut limbs = [Limb::ZERO; LIMBS];

        let nz = CtChoice::from_u32_nonzero(shift);
        let lshift = shift;
        let rshift = nz.if_true_u32(Limb::BITS - shift);
        let carry = nz.if_true_word(self.limbs[LIMBS - 1].0.wrapping_shr(Word::BITS - shift));

        let mut i = LIMBS - 1;
        while i > 0 {
            let mut limb = self.limbs[i].0 << lshift;
            let hi = self.limbs[i - 1].0 >> rshift;
            limb |= nz.if_true_word(hi);
            limbs[i] = Limb(limb);
            i -= 1
        }
        limbs[0] = Limb(self.limbs[0].0 << lshift);

        (Uint::<LIMBS>::new(limbs), Limb(carry))
    }

    /// Computes `self >> 1` in constant-time.
    pub(crate) const fn shl1(&self) -> Self {
        // TODO(tarcieri): optimized implementation
        self.shl_vartime(1)
    }
}

impl<const LIMBS: usize> Shl<u32> for Uint<LIMBS> {
    type Output = Uint<LIMBS>;

    fn shl(self, shift: u32) -> Uint<LIMBS> {
        Uint::<LIMBS>::shl(&self, shift)
    }
}

impl<const LIMBS: usize> Shl<u32> for &Uint<LIMBS> {
    type Output = Uint<LIMBS>;

    fn shl(self, shift: u32) -> Uint<LIMBS> {
        self.shl(shift)
    }
}

impl<const LIMBS: usize> ShlAssign<u32> for Uint<LIMBS> {
    fn shl_assign(&mut self, shift: u32) {
        *self = self.shl(shift)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Limb, Uint, U128, U256};

    const N: U256 =
        U256::from_be_hex("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141");

    const TWO_N: U256 =
        U256::from_be_hex("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFD755DB9CD5E9140777FA4BD19A06C8282");

    const FOUR_N: U256 =
        U256::from_be_hex("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFAEABB739ABD2280EEFF497A3340D90504");

    const SIXTY_FIVE: U256 =
        U256::from_be_hex("FFFFFFFFFFFFFFFD755DB9CD5E9140777FA4BD19A06C82820000000000000000");

    const EIGHTY_EIGHT: U256 =
        U256::from_be_hex("FFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD03641410000000000000000000000");

    const SIXTY_FOUR: U256 =
        U256::from_be_hex("FFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD03641410000000000000000");

    #[test]
    fn shl_simple() {
        let mut t = U256::from(1u8);
        assert_eq!(t << 1, U256::from(2u8));
        t = U256::from(3u8);
        assert_eq!(t << 8, U256::from(0x300u16));
    }

    #[test]
    fn shl1() {
        assert_eq!(N << 1, TWO_N);
    }

    #[test]
    fn shl2() {
        assert_eq!(N << 2, FOUR_N);
    }

    #[test]
    fn shl65() {
        assert_eq!(N << 65, SIXTY_FIVE);
    }

    #[test]
    fn shl88() {
        assert_eq!(N << 88, EIGHTY_EIGHT);
    }

    #[test]
    fn shl256() {
        assert_eq!(N << 256, U256::default());
    }

    #[test]
    fn shl64() {
        assert_eq!(N << 64, SIXTY_FOUR);
    }

    #[test]
    fn shl_wide_1_1_128() {
        assert_eq!(
            Uint::shl_vartime_wide((U128::ONE, U128::ONE), 128),
            (U128::ZERO, U128::ONE)
        );
    }

    #[test]
    fn shl_wide_max_0_1() {
        assert_eq!(
            Uint::shl_vartime_wide((U128::MAX, U128::ZERO), 1),
            (U128::MAX.sbb(&U128::ONE, Limb::ZERO).0, U128::ONE)
        );
    }

    #[test]
    fn shl_wide_max_max_256() {
        assert_eq!(
            Uint::shl_vartime_wide((U128::MAX, U128::MAX), 256),
            (U128::ZERO, U128::ZERO)
        );
    }
}
