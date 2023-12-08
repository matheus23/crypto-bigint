//! Additions between boxed residues.

use super::BoxedResidue;
use core::ops::{Add, AddAssign};

impl<'a> BoxedResidue<'a> {
    /// Adds `rhs`.
    pub fn add(&self, rhs: &Self) -> Self {
        debug_assert_eq!(self.residue_params, rhs.residue_params);

        Self {
            montgomery_form: self
                .montgomery_form
                .add_mod(&rhs.montgomery_form, &self.residue_params.modulus),
            residue_params: self.residue_params,
        }
    }
}

impl<'a> Add<&BoxedResidue<'a>> for &BoxedResidue<'a> {
    type Output = BoxedResidue<'a>;
    fn add(self, rhs: &BoxedResidue<'a>) -> BoxedResidue<'a> {
        self.add(rhs)
    }
}

impl<'a> Add<BoxedResidue<'a>> for &BoxedResidue<'a> {
    type Output = BoxedResidue<'a>;
    #[allow(clippy::op_ref)]
    fn add(self, rhs: BoxedResidue<'a>) -> BoxedResidue<'a> {
        self + &rhs
    }
}

impl<'a> Add<&BoxedResidue<'a>> for BoxedResidue<'a> {
    type Output = BoxedResidue<'a>;
    #[allow(clippy::op_ref)]
    fn add(self, rhs: &BoxedResidue<'a>) -> BoxedResidue<'a> {
        &self + rhs
    }
}

impl<'a> Add<BoxedResidue<'a>> for BoxedResidue<'a> {
    type Output = BoxedResidue<'a>;
    fn add(self, rhs: BoxedResidue<'a>) -> BoxedResidue<'a> {
        &self + &rhs
    }
}

impl<'a> AddAssign<&BoxedResidue<'a>> for BoxedResidue<'a> {
    fn add_assign(&mut self, rhs: &BoxedResidue<'a>) {
        debug_assert_eq!(self.residue_params, rhs.residue_params);
        self.montgomery_form = self
            .montgomery_form
            .add_mod(&rhs.montgomery_form, &self.residue_params.modulus)
    }
}

impl<'a> AddAssign<BoxedResidue<'a>> for BoxedResidue<'a> {
    fn add_assign(&mut self, rhs: BoxedResidue<'a>) {
        *self += &rhs;
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        modular::{BoxedResidue, BoxedResidueParams},
        BoxedUint,
    };
    use hex_literal::hex;

    #[test]
    fn add_overflow() {
        let params = BoxedResidueParams::new(
            BoxedUint::from_be_slice(
                &hex!("ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551"),
                256,
            )
            .unwrap(),
        )
        .unwrap();

        let x = BoxedUint::from_be_slice(
            &hex!("44acf6b7e36c1342c2c5897204fe09504e1e2efb1a900377dbc4e7a6a133ec56"),
            256,
        )
        .unwrap();
        let mut x_mod = BoxedResidue::new(x, &params);

        let y = BoxedUint::from_be_slice(
            &hex!("d5777c45019673125ad240f83094d4252d829516fac8601ed01979ec1ec1a251"),
            256,
        )
        .unwrap();
        let y_mod = BoxedResidue::new(y, &params);

        x_mod += &y_mod;

        let expected = BoxedUint::from_be_slice(
            &hex!("1a2472fde50286541d97ca6a3592dd75beb9c9646e40c511b82496cfc3926956"),
            256,
        )
        .unwrap();

        assert_eq!(expected, x_mod.retrieve());
    }
}
