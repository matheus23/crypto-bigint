//! Negations of boxed residues.

use super::BoxedResidue;
use crate::BoxedUint;
use core::ops::Neg;

impl<'a> BoxedResidue<'a> {
    /// Negates the number.
    pub fn neg(&self) -> Self {
        let zero = Self {
            montgomery_form: BoxedUint::zero_with_precision(self.residue_params.bits_precision()),
            residue_params: self.residue_params,
        };

        zero.sub(self)
    }
}

impl<'a> Neg for BoxedResidue<'a> {
    type Output = Self;
    fn neg(self) -> Self {
        BoxedResidue::neg(&self)
    }
}

impl<'a> Neg for &BoxedResidue<'a> {
    type Output = BoxedResidue<'a>;
    fn neg(self) -> BoxedResidue<'a> {
        BoxedResidue::neg(self)
    }
}
