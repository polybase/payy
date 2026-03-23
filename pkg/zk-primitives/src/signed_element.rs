// lint-long-file-override allow-max-lines=900
use crate::Element;
use ethnum::U256;
use std::cmp::{Ord, Ordering, PartialOrd};
use std::ops::{Add, Div, Mul, Neg, Sub};

/// A signed 256-bit integer that wraps an `Element` with a sign bit
///
/// This type allows performing arithmetic operations with signed numbers,
/// while the underlying `Element` remains unsigned.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct SignedElement {
    /// The absolute value stored as an Element
    pub value: Element,
    /// The sign: true for negative, false for positive or zero
    pub negative: bool,
}

impl SignedElement {
    /// Creates a new SignedElement with the given value and sign
    #[inline]
    #[must_use]
    pub fn new(value: Element, negative: bool) -> Self {
        // Normalize: if value is zero, sign should be positive
        if value.is_zero() {
            Self {
                value,
                negative: false,
            }
        } else {
            Self { value, negative }
        }
    }

    /// The zero element (the additive identity)
    pub const ZERO: Self = Self {
        value: Element::ZERO,
        negative: false,
    };

    /// The one element (the multiplicative identity)
    pub const ONE: Self = Self {
        value: Element::ONE,
        negative: false,
    };

    /// The negative one element
    pub const NEG_ONE: Self = Self {
        value: Element::ONE,
        negative: true,
    };

    /// Get the absolute value of this element
    #[inline]
    #[must_use]
    pub fn abs(self) -> Element {
        self.value
    }

    /// Check if this element is negative
    #[inline]
    #[must_use]
    pub fn is_negative(self) -> bool {
        self.negative
    }

    /// Check if this element is positive (greater than zero)
    #[inline]
    #[must_use]
    pub fn is_positive(self) -> bool {
        !self.negative && !self.value.is_zero()
    }

    /// Check if this element is zero
    #[inline]
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.value.is_zero()
    }

    /// Get the sign of this element: -1, 0, or 1
    #[inline]
    #[must_use]
    pub fn signum(self) -> Self {
        if self.is_zero() {
            Self::ZERO
        } else if self.negative {
            Self::NEG_ONE
        } else {
            Self::ONE
        }
    }

    /// Convert to the underlying U256 value with sign information
    /// Returns the U256 absolute value and a boolean indicating if negative
    #[inline]
    #[must_use]
    pub fn to_u256_with_sign(self) -> (U256, bool) {
        (self.value.to_u256(), self.negative)
    }
}

// Conversions from various types to SignedElement

impl From<Element> for SignedElement {
    fn from(value: Element) -> Self {
        Self::new(value, false)
    }
}

impl From<u64> for SignedElement {
    fn from(value: u64) -> Self {
        Self::new(Element::new(value), false)
    }
}

impl From<i64> for SignedElement {
    fn from(value: i64) -> Self {
        if value < 0 {
            Self::new(Element::new(value.unsigned_abs()), true)
        } else {
            Self::new(Element::new(value.unsigned_abs()), false)
        }
    }
}

// Arithmetic operations

impl Neg for SignedElement {
    type Output = Self;

    fn neg(self) -> Self::Output {
        if self.is_zero() {
            self
        } else {
            Self::new(self.value, !self.negative)
        }
    }
}

impl Add for SignedElement {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        if self.negative == rhs.negative {
            // Same sign, add values and keep the sign
            Self::new(self.value + rhs.value, self.negative)
        } else {
            // Different signs, subtract the smaller from the larger
            match self.value.cmp(&rhs.value) {
                std::cmp::Ordering::Equal => Self::ZERO,
                std::cmp::Ordering::Greater => Self::new(self.value - rhs.value, self.negative),
                std::cmp::Ordering::Less => Self::new(rhs.value - self.value, rhs.negative),
            }
        }
    }
}

impl Sub for SignedElement {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl Mul for SignedElement {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        // Multiply absolute values, XOR the signs
        Self::new(self.value * rhs.value, self.negative != rhs.negative)
    }
}

impl Div for SignedElement {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        // Divide absolute values, XOR the signs
        assert!(!rhs.is_zero(), "Division by zero");
        Self::new(self.value / rhs.value, self.negative != rhs.negative)
    }
}

// Implement operations with Element operands

impl Add<Element> for SignedElement {
    type Output = Self;

    fn add(self, rhs: Element) -> Self::Output {
        self + Self::from(rhs)
    }
}

impl Sub<Element> for SignedElement {
    type Output = Self;

    fn sub(self, rhs: Element) -> Self::Output {
        self - Self::from(rhs)
    }
}

impl Mul<Element> for SignedElement {
    type Output = Self;

    fn mul(self, rhs: Element) -> Self::Output {
        self * Self::from(rhs)
    }
}

impl Div<Element> for SignedElement {
    type Output = Self;

    fn div(self, rhs: Element) -> Self::Output {
        self / Self::from(rhs)
    }
}

// Implement operations with primitive types
impl Add<u64> for SignedElement {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        self + Self::from(rhs)
    }
}

impl Sub<u64> for SignedElement {
    type Output = Self;

    fn sub(self, rhs: u64) -> Self::Output {
        self - Self::from(rhs)
    }
}

impl Mul<u64> for SignedElement {
    type Output = Self;

    fn mul(self, rhs: u64) -> Self::Output {
        self * Self::from(rhs)
    }
}

impl Div<u64> for SignedElement {
    type Output = Self;

    fn div(self, rhs: u64) -> Self::Output {
        self / Self::from(rhs)
    }
}

// Implement operations with i64
impl Add<i64> for SignedElement {
    type Output = Self;

    fn add(self, rhs: i64) -> Self::Output {
        self + Self::from(rhs)
    }
}

impl Sub<i64> for SignedElement {
    type Output = Self;

    fn sub(self, rhs: i64) -> Self::Output {
        self - Self::from(rhs)
    }
}

impl Mul<i64> for SignedElement {
    type Output = Self;

    fn mul(self, rhs: i64) -> Self::Output {
        self * Self::from(rhs)
    }
}

impl Div<i64> for SignedElement {
    type Output = Self;

    fn div(self, rhs: i64) -> Self::Output {
        self / Self::from(rhs)
    }
}

// Display implementation
impl std::fmt::Display for SignedElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.negative {
            write!(f, "-{}", self.value)
        } else {
            write!(f, "{}", self.value)
        }
    }
}

// Implement PartialEq for primitive types
impl PartialEq<Element> for SignedElement {
    fn eq(&self, other: &Element) -> bool {
        !self.negative && self.value == *other
    }
}

impl PartialEq<u64> for SignedElement {
    fn eq(&self, other: &u64) -> bool {
        !self.negative && self.value == *other
    }
}

impl PartialEq<i64> for SignedElement {
    fn eq(&self, other: &i64) -> bool {
        if *other < 0 {
            // Convert negative i64 to positive u64 safely using unsigned_abs() instead of casting
            self.negative && self.value == Element::new(other.unsigned_abs())
        } else {
            // For non-negative values, explicitly create from non-negative i64
            // This should never fail since we've verified other >= 0
            let Ok(u64_value) = u64::try_from(*other) else {
                // This branch should be unreachable - i64::MAX is well within u64 range
                panic!("Failed to convert non-negative i64 {other} to u64");
            };
            !self.negative && self.value == Element::new(u64_value)
        }
    }
}

// Implement a few more utility methods
impl SignedElement {
    /// Tries to convert this SignedElement to an i64
    ///
    /// Returns None if the value doesn't fit in an i64
    #[must_use]
    pub fn to_i64(&self) -> Option<i64> {
        if self.value > Element::new(i64::MAX as u64) {
            None
        } else {
            // First convert to u64
            let unsigned_val = self.value.to_u256().as_u64();

            // Check if it can fit in i64 before conversion
            if !self.negative && unsigned_val > i64::MAX as u64 {
                None
            } else {
                // We've verified the value fits in i64, so conversion should never fail
                let Ok(signed_value) = i64::try_from(unsigned_val) else {
                    // This should be unreachable given our checks
                    panic!("Failed to convert u64 {unsigned_val} to i64 despite being in range");
                };

                Some(if self.negative {
                    -signed_value
                } else {
                    signed_value
                })
            }
        }
    }

    /// Checks if this SignedElement represents a value that fits in an i64
    #[must_use]
    pub fn fits_in_i64(&self) -> bool {
        self.to_i64().is_some()
    }
}

// Implement PartialOrd for SignedElement
impl PartialOrd for SignedElement {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Implement Ord for SignedElement
impl Ord for SignedElement {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            // Both positive (or zero): compare normally
            (false, false) => self.value.cmp(&other.value),

            // Both negative: larger absolute value is smaller
            (true, true) => other.value.cmp(&self.value),

            // Self negative, other positive (or zero): self is less
            (true, false) => Ordering::Less,

            // Self positive (or zero), other negative: self is greater
            (false, true) => Ordering::Greater,
        }
    }
}

// Implement PartialOrd with Element
impl PartialOrd<Element> for SignedElement {
    fn partial_cmp(&self, other: &Element) -> Option<Ordering> {
        if self.negative {
            // Any negative number is less than any Element (which is unsigned)
            Some(Ordering::Less)
        } else {
            // Positive SignedElement: compare values normally
            Some(self.value.cmp(other))
        }
    }
}

// Implement PartialOrd with u64
impl PartialOrd<u64> for SignedElement {
    fn partial_cmp(&self, other: &u64) -> Option<Ordering> {
        if self.negative {
            // Any negative number is less than any u64
            Some(Ordering::Less)
        } else {
            // Positive SignedElement: compare values normally
            let other_element = Element::from(*other);
            Some(self.value.cmp(&other_element))
        }
    }
}

// Implement PartialOrd with i64
impl PartialOrd<i64> for SignedElement {
    fn partial_cmp(&self, other: &i64) -> Option<Ordering> {
        if *other < 0 {
            // Compare with negative i64
            if self.negative {
                // Both negative, compare absolute values (reversed)
                // For negative numbers, the one with the larger absolute value is smaller
                // -15 > -25 because 15 < 25
                let other_abs = other.unsigned_abs();
                let self_abs = self.value.to_u256().as_u64();

                // Here we compare self_abs with other_abs, and then reverse the result
                // because larger absolute value means smaller negative number
                Some(self_abs.cmp(&other_abs).reverse())
            } else {
                // Self positive, other negative: self is greater
                Some(Ordering::Greater)
            }
        } else {
            // Compare with non-negative i64
            if self.negative {
                // Self negative, other non-negative: self is less
                Some(Ordering::Less)
            } else {
                // Both non-negative: normal comparison
                // This conversion should never fail since we've checked other >= 0
                let Ok(u64_other) = u64::try_from(*other) else {
                    // This branch should be unreachable - i64::MAX is well within u64 range
                    panic!("Failed to convert non-negative i64 {other} to u64");
                };
                let other_element = Element::from(u64_other);
                Some(self.value.cmp(&other_element))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_operations() {
        // Test addition with same signs
        let a = SignedElement::from(5i64);
        let b = SignedElement::from(3i64);
        assert_eq!(a + b, SignedElement::from(8i64));
        assert!((a + b).is_positive());

        let a = SignedElement::from(-5i64);
        let b = SignedElement::from(-3i64);
        assert_eq!(a + b, SignedElement::from(-8i64));
        assert!((a + b).is_negative());

        // Test addition with different signs
        let a = SignedElement::from(5i64);
        let b = SignedElement::from(-3i64);
        assert_eq!(a + b, SignedElement::from(2i64));
        assert!((a + b).is_positive());

        let a = SignedElement::from(-5i64);
        let b = SignedElement::from(3i64);
        assert_eq!(a + b, SignedElement::from(-2i64));
        assert!((a + b).is_negative());

        // Test subtraction
        let a = SignedElement::from(5i64);
        let b = SignedElement::from(3i64);
        assert_eq!(a - b, SignedElement::from(2i64));
        assert!((a - b).is_positive());

        let a = SignedElement::from(3i64);
        let b = SignedElement::from(5i64);
        assert_eq!(a - b, SignedElement::from(-2i64));
        assert!((a - b).is_negative());

        // Test multiplication
        let a = SignedElement::from(5i64);
        let b = SignedElement::from(3i64);
        assert_eq!(a * b, SignedElement::from(15i64));
        assert!((a * b).is_positive());

        let a = SignedElement::from(-5i64);
        let b = SignedElement::from(3i64);
        assert_eq!(a * b, SignedElement::from(-15i64));
        assert!((a * b).is_negative());

        let a = SignedElement::from(5i64);
        let b = SignedElement::from(-3i64);
        assert_eq!(a * b, SignedElement::from(-15i64));
        assert!((a * b).is_negative());

        let a = SignedElement::from(-5i64);
        let b = SignedElement::from(-3i64);
        assert_eq!(a * b, SignedElement::from(15i64));
        assert!((a * b).is_positive());

        // Test division
        let a = SignedElement::from(15i64);
        let b = SignedElement::from(3i64);
        assert_eq!(a / b, SignedElement::from(5i64));
        assert!((a / b).is_positive());

        let a = SignedElement::from(-15i64);
        let b = SignedElement::from(3i64);
        assert_eq!(a / b, SignedElement::from(-5i64));
        assert!((a / b).is_negative());

        let a = SignedElement::from(15i64);
        let b = SignedElement::from(-3i64);
        assert_eq!(a / b, SignedElement::from(-5i64));
        assert!((a / b).is_negative());

        let a = SignedElement::from(-15i64);
        let b = SignedElement::from(-3i64);
        assert_eq!(a / b, SignedElement::from(5i64));
        assert!((a / b).is_positive());
    }

    #[test]
    fn test_sign_operations() {
        let a = SignedElement::from(5i64);
        assert!(!a.is_negative());
        assert!(a.is_positive());
        assert!(!a.is_zero());

        let a = SignedElement::from(-5i64);
        assert!(a.is_negative());
        assert!(!a.is_positive());
        assert!(!a.is_zero());

        let a = SignedElement::ZERO;
        assert!(!a.is_negative());
        assert!(!a.is_positive());
        assert!(a.is_zero());

        // Test negation
        let a = SignedElement::from(5i64);
        assert_eq!(-a, SignedElement::from(-5i64));

        let a = SignedElement::from(-5i64);
        assert_eq!(-a, SignedElement::from(5i64));

        let a = SignedElement::ZERO;
        assert_eq!(-a, SignedElement::ZERO);
    }

    #[test]
    fn test_signum() {
        let a = SignedElement::from(5i64);
        assert_eq!(a.signum(), SignedElement::ONE);

        let a = SignedElement::from(-5i64);
        assert_eq!(a.signum(), SignedElement::NEG_ONE);

        let a = SignedElement::ZERO;
        assert_eq!(a.signum(), SignedElement::ZERO);
    }

    #[test]
    fn test_large_numbers() {
        // Create elements with large values near U256 limits
        let max_element = Element::MAX;

        // Create large elements using U256 values
        let large_u256 = U256::from_str_radix(
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0",
            16,
        )
        .unwrap();
        let large_element = Element::from(large_u256);

        // Create signed elements with these large values
        let pos_max = SignedElement::new(max_element, false);
        let neg_max = SignedElement::new(max_element, true);
        let pos_large = SignedElement::new(large_element, false);
        let neg_large = SignedElement::new(large_element, true);

        // Test signs are preserved
        assert!(!pos_max.is_negative());
        assert!(neg_max.is_negative());
        assert!(!pos_large.is_negative());
        assert!(neg_large.is_negative());

        // Test negation of large values
        assert_eq!(-pos_max, neg_max);
        assert_eq!(-neg_max, pos_max);

        // Test addition with large numbers
        let small = SignedElement::from(1u64);

        // Addition with small numbers
        let result = pos_large + small;
        assert!(!result.is_negative());

        // Addition with opposite signs
        let result = pos_large + neg_small();
        assert!(!result.is_negative());

        let result = neg_large + small;
        assert!(result.is_negative());

        // Test multiplication with sign changes
        let result = pos_large * SignedElement::NEG_ONE;
        assert!(result.is_negative());
        assert_eq!(result.value, large_element);

        let result = neg_large * SignedElement::NEG_ONE;
        assert!(!result.is_negative());
        assert_eq!(result.value, large_element);
    }

    // Helper function for a small negative number
    fn neg_small() -> SignedElement {
        SignedElement::new(Element::ONE, true)
    }

    #[test]
    fn test_large_number_sign_preservation() {
        // Create a more moderate large value that won't overflow
        let large_value = Element::from(
            U256::from_str_radix(
                "8000000000000000000000000000000000000000000000000000000000000000",
                16,
            )
            .unwrap(),
        );

        // Test positive large value
        let pos_large = SignedElement::new(large_value, false);
        assert!(!pos_large.is_negative());
        assert!(pos_large.is_positive());

        // Test negative large value
        let neg_large = SignedElement::new(large_value, true);
        assert!(neg_large.is_negative());
        assert!(!neg_large.is_positive());

        // Test negation preserves absolute value
        assert_eq!(-pos_large, neg_large);
        assert_eq!(-neg_large, pos_large);

        // Test addition with small number
        let small = SignedElement::from(10u64);

        // Adding small to positive large
        let result = pos_large + small;
        assert!(!result.is_negative());

        // Adding small to negative large
        let result = neg_large + small;
        // Should still be negative since |large| > |small|
        assert!(result.is_negative());

        // Test subtraction with small number
        let result = pos_large - small;
        // Still positive
        assert!(!result.is_negative());

        let result = neg_large - small;
        // Still negative and absolute value increased
        assert!(result.is_negative());

        // Test with reasonable multiplication
        let small_multiplier = SignedElement::from(2u64);

        let result = pos_large / small_multiplier;
        assert!(!result.is_negative());

        let result = neg_large / small_multiplier;
        assert!(result.is_negative());

        // Test with sign-changing multiplication
        let neg_one = SignedElement::from(-1i64);

        let result = pos_large * neg_one;
        assert!(result.is_negative());

        let result = neg_large * neg_one;
        assert!(!result.is_negative());
    }

    #[test]
    fn test_safe_arithmetic() {
        // Test with values that won't overflow
        let a = SignedElement::from(i64::MAX);
        let b = SignedElement::from(i64::MIN);

        // Test negation
        assert_eq!(-a, SignedElement::from(-i64::MAX));
        // Note: -i64::MIN would overflow in i64, but works in our bigger type
        assert!(!(-b).is_negative());

        // Test addition
        let result = a + a;
        assert!(!result.is_negative());

        let result = b + b;
        assert!(result.is_negative());

        // Test subtraction
        let result = a - b;
        assert!(!result.is_negative());

        // Test multiplication
        let result = a * SignedElement::from(2i64);
        assert!(!result.is_negative());

        let result = b * SignedElement::from(2i64);
        assert!(result.is_negative());

        let result = a * SignedElement::from(-1i64);
        assert!(result.is_negative());

        let result = b * SignedElement::from(-1i64);
        assert!(!result.is_negative());

        // Test with values that could overflow i64 but are fine for Element
        let big_positive = SignedElement::from(u64::MAX);
        let big_negative = SignedElement::new(Element::from(u64::MAX), true);

        assert!(!big_positive.is_negative());
        assert!(big_negative.is_negative());

        // Test sign preservation with division
        let result = big_positive / SignedElement::from(2u64);
        assert!(!result.is_negative());

        let result = big_negative / SignedElement::from(2u64);
        assert!(result.is_negative());

        // Test sign flipping with division
        let result = big_positive / SignedElement::from(-2i64);
        assert!(result.is_negative());

        let result = big_negative / SignedElement::from(-2i64);
        assert!(!result.is_negative());
    }

    #[test]
    fn test_comparison_operators() {
        // Test comparisons between SignedElements with different signs
        let pos = SignedElement::from(5i64);
        let neg = SignedElement::from(-5i64);
        let zero = SignedElement::ZERO;

        // Positive vs Negative
        assert!(pos > neg);
        assert!(neg < pos);
        assert!(pos >= neg);
        assert!(neg <= pos);
        assert!(pos != neg);

        // Zero vs Positive/Negative
        assert!(zero > neg);
        assert!(zero < pos);
        assert!(zero >= neg);
        assert!(zero <= pos);
        assert!(zero != pos);
        assert!(zero != neg);

        // Test comparisons between SignedElements with the same sign
        let pos_small = SignedElement::from(3i64);
        let pos_large = SignedElement::from(10i64);
        let neg_small = SignedElement::from(-3i64);
        let neg_large = SignedElement::from(-10i64);

        // Positive comparisons
        assert!(pos_large > pos_small);
        assert!(pos_small < pos_large);
        assert!(pos_large >= pos_small);
        assert!(pos_small <= pos_large);
        assert!(pos_large != pos_small);

        // Negative comparisons (larger absolute value is smaller)
        assert!(neg_large < neg_small);
        assert!(neg_small > neg_large);
        assert!(neg_large <= neg_small);
        assert!(neg_small >= neg_large);
        assert!(neg_large != neg_small);

        // Equal values
        let pos_equal1 = SignedElement::from(7i64);
        let pos_equal2 = SignedElement::from(7i64);
        let neg_equal1 = SignedElement::from(-7i64);
        let neg_equal2 = SignedElement::from(-7i64);

        assert_eq!(pos_equal1, pos_equal2);
        assert!(pos_equal1 >= pos_equal2);
        assert!(pos_equal1 <= pos_equal2);
        assert!(pos_equal1 <= pos_equal2);
        assert!(pos_equal1 >= pos_equal2);

        assert_eq!(neg_equal1, neg_equal2);
        assert!(neg_equal1 >= neg_equal2);
        assert!(neg_equal1 <= neg_equal2);
        assert!(neg_equal1 <= neg_equal2);
        assert!(neg_equal1 >= neg_equal2);

        // Test comparisons with Element
        let pos_elem = SignedElement::from(15i64);
        let neg_elem = SignedElement::from(-15i64);
        let elem = Element::new(15);

        assert!(pos_elem == elem);
        assert!(neg_elem < elem);
        assert!(neg_elem <= elem);
        assert!(pos_elem >= elem);
        assert!(pos_elem >= elem);
        assert!(pos_elem <= elem);

        // Test comparisons with u64
        let u64_val = 20u64;
        let pos_u64 = SignedElement::from(20i64);
        let equal_u64 = SignedElement::from(20i64);
        let neg_u64 = SignedElement::from(-20i64);
        let smaller_u64 = SignedElement::from(10i64);
        let larger_u64 = SignedElement::from(30i64);

        assert!(pos_u64 == u64_val);
        assert!(equal_u64 == u64_val);
        assert!(neg_u64 < u64_val);
        assert!(smaller_u64 < u64_val);
        assert!(larger_u64 > u64_val);

        // Test comparisons with i64
        let i64_pos = 25i64;
        let i64_neg = -25i64;
        let equal_pos = SignedElement::from(25i64);
        let equal_neg = SignedElement::from(-25i64);
        let smaller_pos = SignedElement::from(15i64);
        let smaller_neg = SignedElement::from(-15i64);
        let larger_pos = SignedElement::from(35i64);
        let larger_neg = SignedElement::from(-35i64);

        // Compare with positive i64
        assert!(equal_pos == i64_pos);
        assert!(equal_neg < i64_pos);
        assert!(smaller_pos < i64_pos);
        assert!(larger_pos > i64_pos);

        // Compare with negative i64
        assert!(equal_neg == i64_neg);
        assert!(equal_pos > i64_neg);
        assert!(smaller_neg > i64_neg); // -15 > -25
        assert!(larger_neg < i64_neg); // -35 < -25
    }
}
