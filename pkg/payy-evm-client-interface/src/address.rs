// lint-long-file-override allow-max-lines=300
use bn254_blackbox_solver::multi_scalar_mul;
use contextful::ResultContextExt;
use element::Element;
use hash::hash_merge;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};

use crate::evm::B256;
use crate::{Error, Result, ValidationErrorKind};

const SIGN_MASK: u8 = 0x80;
const RESERVED_MASK: u8 = 0x40;
const GRUMPKIN_B: u8 = 17;
const GRUMPKIN_GENERATOR_Y: B256 = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xcf, 0x13, 0x5e, 0x75, 0x06, 0xa4, 0x5d, 0x63,
    0x2d, 0x27, 0x0d, 0x45, 0xf1, 0x18, 0x12, 0x94, 0x83, 0x3f, 0xc4, 0x8d, 0x82, 0x3f, 0x27, 0x2c,
];

/// Compact Grumpkin public-key address used by Payy privacy flows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PrivacyAddress {
    /// Canonical 32-byte compact public key.
    pub bytes: [u8; 32],
}

impl PrivacyAddress {
    /// Construct from raw compact bytes.
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// Construct the canonical compact address for a Grumpkin public key.
    pub fn from_public_key(public_key_x: Element, public_key_y: Element) -> Result<Self> {
        validate_point(public_key_x, public_key_y)?;
        let mut bytes = public_key_x.to_be_bytes();
        if public_key_y.to_be_bytes()[31] & 1 == 1 {
            bytes[0] |= SIGN_MASK;
        }
        Ok(Self { bytes })
    }

    /// Decode the compact public key into affine coordinates.
    pub fn public_key(self) -> Result<(Element, Element)> {
        if self.bytes[0] & RESERVED_MASK != 0 {
            return Err(Error::Validation {
                kind: ValidationErrorKind::InvalidPrivacyAddress,
            });
        }
        let sign_is_odd = self.bytes[0] & SIGN_MASK != 0;
        let mut x_bytes = self.bytes;
        x_bytes[0] &= !(SIGN_MASK | RESERVED_MASK);
        let x = Element::from_be_bytes(x_bytes);
        if x >= Element::MODULUS {
            return Err(Error::Validation {
                kind: ValidationErrorKind::FieldOutOfRange,
            });
        }
        let y = recover_y(x, sign_is_odd)?;
        Ok((x, y))
    }

    /// Return the owner hash derived from this address.
    pub fn owner(self) -> Result<Element> {
        let (public_key_x, public_key_y) = self.public_key()?;
        Ok(owner_from_public_key(public_key_x, public_key_y))
    }

    /// Return the fixed 6-byte receive prefix.
    pub fn prefix(self) -> Result<PrivacyAddressPrefix> {
        let owner = self.owner()?.to_be_bytes();
        Ok(PrivacyAddressPrefix {
            bytes: [owner[0], owner[1], owner[2], owner[3], owner[4], owner[5]],
        })
    }
}

/// Fixed-width incoming-note discovery prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PrivacyAddressPrefix {
    /// Exact 6-byte prefix. Leading zeros are significant.
    pub bytes: [u8; 6],
}

/// Compute a Grumpkin public key from a private scalar.
pub fn grumpkin_public_key_from_private_key(private_key: B256) -> Result<(Element, Element)> {
    grumpkin_scalar_mul_point(
        private_key,
        Element::ONE,
        Element::from_be_bytes(GRUMPKIN_GENERATOR_Y),
    )
}

/// Multiply a Grumpkin point by a private scalar.
pub fn grumpkin_scalar_mul_point(
    private_key: B256,
    public_key_x: Element,
    public_key_y: Element,
) -> Result<(Element, Element)> {
    let private_key = Element::from_be_bytes(private_key);
    let private_key_bytes = private_key.to_be_bytes();
    let mut hi_bytes = [0u8; 16];
    hi_bytes.copy_from_slice(&private_key_bytes[..16]);
    let mut lo_bytes = [0u8; 16];
    lo_bytes.copy_from_slice(&private_key_bytes[16..]);
    let scalar_hi = u128::from_be_bytes(hi_bytes);
    let scalar_lo = u128::from_be_bytes(lo_bytes);
    let (x, y, _) = multi_scalar_mul(
        &[
            public_key_x.to_base(),
            public_key_y.to_base(),
            Element::ZERO.to_base(),
        ],
        &[Element::from(scalar_lo).to_base()],
        &[Element::from(scalar_hi).to_base()],
        true,
    )
    .context("compute grumpkin public key")?;
    Ok((Element::from_base(x), Element::from_base(y)))
}

/// Compute the canonical owner hash for a Grumpkin public key.
#[must_use]
pub fn owner_from_public_key(public_key_x: Element, public_key_y: Element) -> Element {
    hash_merge([public_key_x, public_key_y])
}

/// Compute the canonical owner hash for a Grumpkin private scalar.
pub fn owner_from_private_key(private_key: B256) -> Result<Element> {
    let (public_key_x, public_key_y) = grumpkin_public_key_from_private_key(private_key)?;
    Ok(owner_from_public_key(public_key_x, public_key_y))
}

fn validate_point(public_key_x: Element, public_key_y: Element) -> Result<()> {
    if public_key_x >= Element::MODULUS || public_key_y >= Element::MODULUS {
        return Err(Error::Validation {
            kind: ValidationErrorKind::FieldOutOfRange,
        });
    }
    let expected_y = recover_y(public_key_x, public_key_y.to_be_bytes()[31] & 1 == 1)?;
    if expected_y != public_key_y {
        return Err(Error::Validation {
            kind: ValidationErrorKind::InvalidPrivacyAddress,
        });
    }
    Ok(())
}

fn recover_y(public_key_x: Element, sign_is_odd: bool) -> Result<Element> {
    let modulus = modulus_biguint();
    let x = BigUint::from_bytes_be(&public_key_x.to_be_bytes());
    let rhs = (&x * &x * &x + &modulus - BigUint::from(GRUMPKIN_B)) % &modulus;
    let mut y = mod_sqrt(&rhs, &modulus).ok_or(Error::Validation {
        kind: ValidationErrorKind::InvalidPrivacyAddress,
    })?;
    if (&y & BigUint::one()).is_one() != sign_is_odd {
        y = &modulus - y;
    }
    let mut bytes = [0u8; 32];
    let y_bytes = y.to_bytes_be();
    if y_bytes.len() > bytes.len() {
        return Err(Error::Validation {
            kind: ValidationErrorKind::InvalidPrivacyAddress,
        });
    }
    let start = bytes.len() - y_bytes.len();
    bytes[start..].copy_from_slice(&y_bytes);
    Ok(Element::from_be_bytes(bytes))
}

#[allow(clippy::many_single_char_names)]
fn mod_sqrt(value: &BigUint, modulus: &BigUint) -> Option<BigUint> {
    if value.is_zero() {
        return Some(BigUint::zero());
    }
    let one = BigUint::one();
    let two = BigUint::from(2u8);
    let legendre_exp = (modulus - &one) >> 1usize;
    if value.modpow(&legendre_exp, modulus) != one {
        return None;
    }
    let mut q = modulus - &one;
    let mut s = 0u32;
    while (&q & &one).is_zero() {
        q >>= 1usize;
        s += 1;
    }
    let mut z = two.clone();
    while z.modpow(&legendre_exp, modulus) != modulus - &one {
        z += &one;
    }
    let mut m = s;
    let mut c = z.modpow(&q, modulus);
    let mut t = value.modpow(&q, modulus);
    let mut r = value.modpow(&((&q + &one) >> 1usize), modulus);
    while t != one {
        let mut i = 1u32;
        let mut t2i = (&t * &t) % modulus;
        while t2i != one {
            t2i = (&t2i * &t2i) % modulus;
            i += 1;
            if i == m {
                return None;
            }
        }
        let shift = usize::try_from(m - i - 1).ok()?;
        let b = c.modpow(&(BigUint::one() << shift), modulus);
        r = (&r * &b) % modulus;
        c = (&b * &b) % modulus;
        t = (&t * &c) % modulus;
        m = i;
    }
    Some(r)
}

fn modulus_biguint() -> BigUint {
    BigUint::from_bytes_be(&Element::MODULUS.to_be_bytes())
}
