//! Negations of residues with a modulus set at runtime.

use super::DynResidue;
use core::ops::Neg;

impl<const LIMBS: usize> DynResidue<LIMBS> {
    /// Negates the number.
    pub const fn neg(&self) -> Self {
        Self::zero(self.residue_params).sub(self)
    }
}

impl<const LIMBS: usize> Neg for DynResidue<LIMBS> {
    type Output = Self;
    fn neg(self) -> Self {
        DynResidue::neg(&self)
    }
}

impl<const LIMBS: usize> Neg for &DynResidue<LIMBS> {
    type Output = DynResidue<LIMBS>;
    fn neg(self) -> DynResidue<LIMBS> {
        DynResidue::neg(self)
    }
}
