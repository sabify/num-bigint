use std::cmp;
use std::cmp::Ordering::*;

use num_traits::Zero;
use smallvec::SmallVec;

use crate::algorithms::cmp_slice;
use crate::big_digit::{BigDigit, SignedDoubleBigDigit, BITS};
use crate::bigint::Sign::{self, *};
use crate::{BigUint, VEC_SIZE};

/// Subtract with borrow:
#[inline]
pub fn sbb(a: BigDigit, b: BigDigit, acc: &mut SignedDoubleBigDigit) -> BigDigit {
    *acc += a as SignedDoubleBigDigit;
    *acc -= b as SignedDoubleBigDigit;
    let lo = *acc as BigDigit;
    *acc >>= BITS;
    lo
}

#[inline]
fn is_zero(x: &[BigDigit]) -> bool {
    if x.is_empty() {
        return true;
    }

    x.iter().all(|xi| *xi == 0)
}

#[inline]
pub fn __sub2(a: &mut [BigDigit], b: &[BigDigit]) -> bool {
    let mut borrow = 0;

    if b.len() == 1 {
        return __sub_scalar(a, b[0]);
    }
    if is_zero(&*a) {
        return !is_zero(b);
    }

    let len = cmp::min(a.len(), b.len());
    let (a_lo, a_hi) = a.split_at_mut(len);
    let (b_lo, b_hi) = b.split_at(len);

    for (a, b) in a_lo.iter_mut().zip(b_lo) {
        *a = sbb(*a, *b, &mut borrow);
    }

    if borrow != 0 {
        for a in a_hi {
            *a = sbb(*a, 0, &mut borrow);
            if borrow == 0 {
                break;
            }
        }
    }

    borrow != 0
}

/// Calculate `a -= b`. Fails on underflow.
#[inline]
pub fn sub2(a: &mut [BigDigit], b: &[BigDigit]) {
    let borrow = __sub2(a, b);

    // note: we're _required_ to fail on underflow
    assert!(
        !borrow,
        "Cannot subtract b from a because b is larger than a.\na: {:?}\nb: {:?}",
        a, b
    );
}

/// Calculate `a -= b` for `b` a scalar.
#[inline]
pub fn __sub_scalar(a: &mut [BigDigit], b: BigDigit) -> bool {
    let mut bw = b;
    for ai in a.iter_mut() {
        let (tmp, overflow) = ai.overflowing_sub(bw);
        bw = overflow as BigDigit;
        *ai = tmp;

        if !overflow {
            break;
        }
    }

    bw > 0
}

// Only for the SubRev impl. `a` and `b` must have same length.
#[inline]
pub fn __sub2rev(a: &[BigDigit], b: &mut [BigDigit]) -> bool {
    debug_assert!(b.len() == a.len());

    let mut borrow = 0;

    for (ai, bi) in a.iter().zip(b) {
        *bi = sbb(*ai, *bi, &mut borrow);
    }

    borrow != 0
}

pub fn sub2rev(a: &[BigDigit], b: &mut [BigDigit]) {
    debug_assert!(b.len() >= a.len());

    let len = cmp::min(a.len(), b.len());
    let (a_lo, a_hi) = a.split_at(len);
    let (b_lo, b_hi) = b.split_at_mut(len);

    let borrow = __sub2rev(a_lo, b_lo);

    assert!(a_hi.is_empty());

    // note: we're _required_ to fail on underflow
    assert!(
        !borrow && b_hi.iter().all(|x| *x == 0),
        "Cannot subtract b from a because b is larger than a."
    );
}

pub fn sub_sign(a: &[BigDigit], b: &[BigDigit]) -> (Sign, BigUint) {
    // Normalize:
    let a = &a[..a.iter().rposition(|&x| x != 0).map_or(0, |i| i + 1)];
    let b = &b[..b.iter().rposition(|&x| x != 0).map_or(0, |i| i + 1)];

    match cmp_slice(a, b) {
        Greater => {
            let mut a: SmallVec<[BigDigit; VEC_SIZE]> = a.into();
            sub2(&mut a, b);
            (Plus, BigUint::new_native(a))
        }
        Less => {
            let mut b: SmallVec<[BigDigit; VEC_SIZE]> = b.into();
            sub2(&mut b, a);
            (Minus, BigUint::new_native(b))
        }
        _ => (NoSign, Zero::zero()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use num_traits::Num;

    use crate::BigInt;

    #[test]
    fn test_sub_sign() {
        fn sub_sign_i(a: &[BigDigit], b: &[BigDigit]) -> BigInt {
            let (sign, val) = sub_sign(a, b);
            BigInt::from_biguint(sign, val)
        }

        let a = BigUint::from_str_radix("265252859812191058636308480000000", 10).unwrap();
        let b = BigUint::from_str_radix("26525285981219105863630848000000", 10).unwrap();
        let a_i = BigInt::from_biguint(Plus, a.clone());
        let b_i = BigInt::from_biguint(Plus, b.clone());

        assert_eq!(sub_sign_i(&a.data[..], &b.data[..]), &a_i - &b_i);
        assert_eq!(sub_sign_i(&b.data[..], &a.data[..]), &b_i - &a_i);
    }
}
