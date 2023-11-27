//! Equivalence tests between `crypto_bigint::BoxedUint` and `num_bigint::BigUint`.

#![cfg(feature = "alloc")]

use crypto_bigint::{BoxedUint, CheckedAdd, Limb};
use num_bigint::BigUint;
use proptest::prelude::*;

fn to_biguint(uint: &BoxedUint) -> BigUint {
    BigUint::from_bytes_be(&uint.to_be_bytes())
}

fn to_uint(big_uint: BigUint) -> BoxedUint {
    let bytes = big_uint.to_bytes_be();
    let pad_count = Limb::BYTES - (bytes.len() % Limb::BYTES);
    let mut padded_bytes = vec![0u8; pad_count];
    padded_bytes.extend_from_slice(&bytes);
    BoxedUint::from_be_slice(&padded_bytes, padded_bytes.len() * 8).unwrap()
}

prop_compose! {
    /// Generate a random `BoxedUint`.
    fn uint()(mut bytes in any::<Vec<u8>>()) -> BoxedUint {
        let extra = bytes.len() % Limb::BYTES;
        let bytes_precision = bytes.len() - extra;
        bytes.truncate(bytes_precision);
        BoxedUint::from_be_slice(&bytes, bytes_precision * 8).unwrap()
    }
}
prop_compose! {
    /// Generate a pair of random `BoxedUint`s with the same precision.
    fn uint_pair()(mut a in uint(), mut b in uint()) -> (BoxedUint, BoxedUint) {
        if a.bits_precision() > b.bits_precision() {
            b = b.widen(a.bits_precision());
        } else if a.bits_precision() < b.bits_precision() {
            a = a.widen(b.bits_precision());
        }

        (a, b)
    }
}

proptest! {
    #[test]
    fn roundtrip(a in uint()) {
        prop_assert_eq!(&a, &to_uint(to_biguint(&a)));
    }

    #[test]
    fn checked_add(a in uint(), b in uint()) {
        let a_bi = to_biguint(&a);
        let b_bi = to_biguint(&b);
        let expected = a_bi + b_bi;

        match Option::<BoxedUint>::from(a.checked_add(&b)) {
            Some(actual) => prop_assert_eq!(expected, to_biguint(&actual)),
            None => {
                let max = BoxedUint::max(a.bits_precision());
                prop_assert!(expected > to_biguint(&max));
            }
        }
    }

    #[test]
    fn mul_wide(a in uint(), b in uint()) {
        let a_bi = to_biguint(&a);
        let b_bi = to_biguint(&b);

        let expected = a_bi * b_bi;
        let actual = a.mul_wide(&b);

        prop_assert_eq!(expected, to_biguint(&actual));
    }

    #[test]
    fn rem_vartime((a, b) in uint_pair()) {
        if bool::from(!b.is_zero()) {
            let a_bi = to_biguint(&a);
            let b_bi = to_biguint(&b);

            let expected = a_bi % b_bi;
            let actual = a.rem_vartime(&b).unwrap();

            prop_assert_eq!(expected, to_biguint(&actual));
        }
    }
}
