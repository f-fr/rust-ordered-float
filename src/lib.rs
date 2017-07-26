#![cfg_attr(test, deny(warnings))]
#![deny(missing_docs)]

//! Wrappers for total order on Floats.

extern crate num_traits;
extern crate unreachable;

use std::cmp::Ordering;
use std::error::Error;
use std::ops::{Add, AddAssign, Deref, DerefMut, Div, DivAssign, Mul, MulAssign, Neg, Rem,
               RemAssign, Sub, SubAssign};
use std::hash::{Hash, Hasher};
use std::fmt;
use std::io;
use std::mem;
use unreachable::unreachable;
use num_traits::{Bounded, Float, FromPrimitive, Num, One, Signed, ToPrimitive, Zero};

// masks for the parts of the IEEE 754 float
const SIGN_MASK: u64 = 0x8000000000000000u64;
const EXP_MASK: u64 = 0x7ff0000000000000u64;
const MAN_MASK: u64 = 0x000fffffffffffffu64;

// canonical raw bit patterns (for hashing)
const CANONICAL_NAN_BITS: u64 = 0x7ff8000000000000u64;
const CANONICAL_ZERO_BITS: u64 = 0x0u64;

/// A wrapper around Floats providing an implementation of Ord and Hash.
///
/// NaN is sorted as *greater* than all other values and *equal*
/// to itself, in contradiction with the IEEE standard.
#[derive(PartialOrd, Debug, Default, Clone, Copy)]
pub struct OrderedFloat<T: Float>(pub T);

impl<T: Float> OrderedFloat<T> {
    /// Get the value out.
    pub fn into_inner(self) -> T {
        let OrderedFloat(val) = self;
        val
    }
}

impl<T: Float> AsRef<T> for OrderedFloat<T> {
    fn as_ref(&self) -> &T {
        let OrderedFloat(ref val) = *self;
        val
    }
}

impl<T: Float> AsMut<T> for OrderedFloat<T> {
    fn as_mut(&mut self) -> &mut T {
        let OrderedFloat(ref mut val) = *self;
        val
    }
}

impl<T: Float + PartialOrd> Ord for OrderedFloat<T> {
    fn cmp(&self, other: &OrderedFloat<T>) -> Ordering {
        match self.partial_cmp(&other) {
            Some(ordering) => ordering,
            None => {
                if self.as_ref().is_nan() {
                    if other.as_ref().is_nan() {
                        Ordering::Equal
                    } else {
                        Ordering::Greater
                    }
                } else {
                    Ordering::Less
                }
            }
        }
    }
}

impl<T: Float + PartialEq> PartialEq for OrderedFloat<T> {
    fn eq(&self, other: &OrderedFloat<T>) -> bool {
        if self.as_ref().is_nan() {
            if other.as_ref().is_nan() { true } else { false }
        } else if other.as_ref().is_nan() {
            false
        } else {
            self.as_ref() == other.as_ref()
        }
    }
}

impl<T: Float> Hash for OrderedFloat<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.is_nan() {
            // normalize to one representation of NaN
            hash_float(&T::nan(), state)
        } else {
            hash_float(self.as_ref(), state)
        }
    }
}

impl<T: Float + fmt::Display> fmt::Display for OrderedFloat<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl Into<f32> for OrderedFloat<f32> {
    fn into(self) -> f32 {
        self.into_inner()
    }
}

impl Into<f64> for OrderedFloat<f64> {
    fn into(self) -> f64 {
        self.into_inner()
    }
}

impl<T: Float> From<T> for OrderedFloat<T> {
    fn from(val: T) -> Self {
        OrderedFloat(val)
    }
}

impl<T: Float> Deref for OrderedFloat<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: Float> DerefMut for OrderedFloat<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T: Float + PartialEq> Eq for OrderedFloat<T> {}

/// A wrapper around Floats providing an implementation of Ord and Hash.
///
/// A NaN value cannot be stored in this type.
#[derive(PartialOrd, PartialEq, Debug, Default, Clone, Copy)]
pub struct NotNaN<T: Float>(T);

impl<T: Float> NotNaN<T> {
    /// Create a NotNaN value.
    ///
    /// Returns Err if val is NaN
    pub fn new(val: T) -> Result<Self, FloatIsNaN> {
        match val {
            ref val if val.is_nan() => Err(FloatIsNaN),
            val => Ok(NotNaN(val)),
        }
    }

    /// Create a NotNaN value from a value that is guaranteed to not be NaN
    ///
    /// Behaviour is undefined if `val` is NaN
    pub unsafe fn unchecked_new(val: T) -> Self {
        debug_assert!(!val.is_nan());
        NotNaN(val)
    }

    /// Get the value out.
    pub fn into_inner(self) -> T {
        let NotNaN(val) = self;
        val
    }
}

impl<T: Float> AsRef<T> for NotNaN<T> {
    fn as_ref(&self) -> &T {
        let NotNaN(ref val) = *self;
        val
    }
}

impl<T: Float + PartialOrd> Ord for NotNaN<T> {
    fn cmp(&self, other: &NotNaN<T>) -> Ordering {
        match self.partial_cmp(&other) {
            Some(ord) => ord,
            None => unsafe { unreachable() },
        }
    }
}

impl<T: Float> Hash for NotNaN<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        hash_float(self.as_ref(), state)
    }
}

impl<T: Float + fmt::Display> fmt::Display for NotNaN<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

impl Into<f32> for NotNaN<f32> {
    fn into(self) -> f32 {
        self.into_inner()
    }
}

impl Into<f64> for NotNaN<f64> {
    fn into(self) -> f64 {
        self.into_inner()
    }
}

/// Creates a NotNaN value from a Float.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl<T: Float> From<T> for NotNaN<T> {
    fn from(v: T) -> Self {
        assert!(!v.is_nan());
        NotNaN(v)
    }
}

impl<T: Float> Deref for NotNaN<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: Float + PartialEq> Eq for NotNaN<T> {}

impl<T: Float> Add for NotNaN<T> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        NotNaN(self.0 + other.0)
    }
}

/// Adds a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl<T: Float> Add<T> for NotNaN<T> {
    type Output = Self;

    fn add(self, other: T) -> Self {
        assert!(!other.is_nan());
        NotNaN::new(self.0 + other).expect("Addition resulted in NaN")
    }
}

impl AddAssign for NotNaN<f64> {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
        assert!(!self.0.is_nan(), "Addition resulted in NaN")
    }
}

impl AddAssign for NotNaN<f32> {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
        assert!(!self.0.is_nan(), "Addition resulted in NaN")
    }
}

/// Adds a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl AddAssign<f64> for NotNaN<f64> {
    fn add_assign(&mut self, other: f64) {
        assert!(!other.is_nan());
        self.0 += other;
        assert!(!self.0.is_nan(), "Addition resulted in NaN")
    }
}

/// Adds a float directly.
///
/// Panics if the provided value is NaN.
impl AddAssign<f32> for NotNaN<f32> {
    fn add_assign(&mut self, other: f32) {
        assert!(!other.is_nan());
        self.0 += other;
        assert!(!self.0.is_nan(), "Addition resulted in NaN")
    }
}

impl<T: Float> Sub for NotNaN<T> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        NotNaN::new(self.0 - other.0).expect("Subtraction resulted in NaN")
    }
}

/// Subtracts a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl<T: Float> Sub<T> for NotNaN<T> {
    type Output = Self;

    fn sub(self, other: T) -> Self {
        assert!(!other.is_nan());
        NotNaN::new(self.0 - other).expect("Subtraction resulted in NaN")
    }
}

impl SubAssign for NotNaN<f64> {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
        assert!(!self.0.is_nan(), "Subtraction resulted in NaN")
    }
}

impl SubAssign for NotNaN<f32> {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
        assert!(!self.0.is_nan(), "Subtraction resulted in NaN")
    }
}

/// Subtracts a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl SubAssign<f64> for NotNaN<f64> {
    fn sub_assign(&mut self, other: f64) {
        assert!(!other.is_nan());
        self.0 -= other;
        assert!(!self.0.is_nan(), "Subtraction resulted in NaN")
    }
}

/// Subtracts a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl SubAssign<f32> for NotNaN<f32> {
    fn sub_assign(&mut self, other: f32) {
        assert!(!other.is_nan());
        self.0 -= other;
        assert!(!self.0.is_nan(), "Subtraction resulted in NaN")
    }
}

impl<T: Float> Mul for NotNaN<T> {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        NotNaN::new(self.0 * other.0).expect("Multiplication resulted in NaN")
    }
}

/// Multiplies a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl<T: Float> Mul<T> for NotNaN<T> {
    type Output = Self;

    fn mul(self, other: T) -> Self {
        assert!(!other.is_nan());
        NotNaN::new(self.0 * other).expect("Multiplication resulted in NaN")
    }
}

impl MulAssign for NotNaN<f64> {
    fn mul_assign(&mut self, other: Self) {
        self.0 *= other.0;
        assert!(!self.0.is_nan(), "Multiplication resulted in NaN")
    }
}

impl MulAssign for NotNaN<f32> {
    fn mul_assign(&mut self, other: Self) {
        self.0 *= other.0;
        assert!(!self.0.is_nan(), "Multiplication resulted in NaN")
    }
}

/// Multiplies a float directly.
///
/// Panics if the provided value is NaN.
impl MulAssign<f64> for NotNaN<f64> {
    fn mul_assign(&mut self, other: f64) {
        assert!(!other.is_nan());
        self.0 *= other;
    }
}

/// Multiplies a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl MulAssign<f32> for NotNaN<f32> {
    fn mul_assign(&mut self, other: f32) {
        assert!(!other.is_nan());
        self.0 *= other;
        assert!(!self.0.is_nan(), "Multiplication resulted in NaN")
    }
}

impl<T: Float> Div for NotNaN<T> {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        NotNaN::new(self.0 / other.0).expect("Division resulted in NaN")
    }
}

/// Divides a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl<T: Float> Div<T> for NotNaN<T> {
    type Output = Self;

    fn div(self, other: T) -> Self {
        assert!(!other.is_nan());
        NotNaN::new(self.0 / other).expect("Division resulted in NaN")
    }
}

impl DivAssign for NotNaN<f64> {
    fn div_assign(&mut self, other: Self) {
        self.0 /= other.0;
        assert!(!self.0.is_nan(), "Division resulted in NaN")
    }
}

impl DivAssign for NotNaN<f32> {
    fn div_assign(&mut self, other: Self) {
        self.0 /= other.0;
        assert!(!self.0.is_nan(), "Division resulted in NaN")
    }
}

/// Divides a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl DivAssign<f64> for NotNaN<f64> {
    fn div_assign(&mut self, other: f64) {
        assert!(!other.is_nan());
        self.0 /= other;
        assert!(!self.0.is_nan(), "Division resulted in NaN")
    }
}

/// Divides a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl DivAssign<f32> for NotNaN<f32> {
    fn div_assign(&mut self, other: f32) {
        assert!(!other.is_nan());
        self.0 /= other;
        assert!(!self.0.is_nan(), "Division resulted in NaN")
    }
}

impl<T: Float> Rem for NotNaN<T> {
    type Output = Self;

    fn rem(self, other: Self) -> Self {
        NotNaN::new(self.0 % other.0).expect("Rem resulted in NaN")
    }
}

/// Calculates `%` with a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl<T: Float> Rem<T> for NotNaN<T> {
    type Output = Self;

    fn rem(self, other: T) -> Self {
        assert!(!other.is_nan());
        NotNaN::new(self.0 % other).expect("Rem resulted in NaN")
    }
}

impl RemAssign for NotNaN<f64> {
    fn rem_assign(&mut self, other: Self) {
        self.0 %= other.0;
        assert!(!self.0.is_nan(), "Rem resulted in NaN")
    }
}

impl RemAssign for NotNaN<f32> {
    fn rem_assign(&mut self, other: Self) {
        self.0 %= other.0;
        assert!(!self.0.is_nan(), "Rem resulted in NaN")
    }
}

/// Calculates `%=` with a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl RemAssign<f64> for NotNaN<f64> {
    fn rem_assign(&mut self, other: f64) {
        assert!(!other.is_nan());
        self.0 %= other;
        assert!(!self.0.is_nan(), "Rem resulted in NaN")
    }
}

/// Calculates `%=` with a float directly.
///
/// Panics if the provided value is NaN or the computation results in NaN
impl RemAssign<f32> for NotNaN<f32> {
    fn rem_assign(&mut self, other: f32) {
        assert!(!other.is_nan());
        self.0 %= other;
        assert!(!self.0.is_nan(), "Rem resulted in NaN")
    }
}

impl<T: Float> Neg for NotNaN<T> {
    type Output = Self;

    fn neg(self) -> Self {
        NotNaN::new(-self.0).expect("Negation resulted in NaN")
    }
}

/// An error indicating an attempt to construct NotNaN from a NaN
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FloatIsNaN;

impl Error for FloatIsNaN {
    fn description(&self) -> &str {
        return "NotNaN constructed with NaN";
    }
}

impl fmt::Display for FloatIsNaN {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl Into<io::Error> for FloatIsNaN {
    fn into(self) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidInput, self)
    }
}

#[inline]
fn hash_float<F: Float, H: Hasher>(f: &F, state: &mut H) {
    raw_double_bits(f).hash(state);
}

#[inline]
fn raw_double_bits<F: Float>(f: &F) -> u64 {
    if f.is_nan() {
        return CANONICAL_NAN_BITS;
    }

    let (man, exp, sign) = f.integer_decode();
    if man == 0 {
        return CANONICAL_ZERO_BITS;
    }

    let exp_u64 = unsafe { mem::transmute::<i16, u16>(exp) } as u64;
    let sign_u64 = if sign > 0 { 1u64 } else { 0u64 };
    (man & MAN_MASK) | ((exp_u64 << 52) & EXP_MASK) | ((sign_u64 << 63) & SIGN_MASK)
}

impl<T: Float + Zero> Zero for NotNaN<T> {
    fn zero() -> Self { NotNaN(T::zero()) }

    fn is_zero(&self) -> bool { self.0.is_zero() }
}

impl<T: Float + One> One for NotNaN<T> {
    fn one() -> Self { NotNaN(T::one()) }
}

impl<T: Float + Bounded> Bounded for NotNaN<T> {
    fn min_value() -> Self {
        NotNaN(Bounded::min_value())
    }

    fn max_value() -> Self {
        NotNaN(Bounded::max_value())
    }
}

impl<T: Float + FromPrimitive> FromPrimitive for NotNaN<T> {
    fn from_i64(n: i64) -> Option<Self> { T::from_i64(n).and_then(|n| NotNaN::new(n).ok()) }
    fn from_u64(n: u64) -> Option<Self> { T::from_u64(n).and_then(|n| NotNaN::new(n).ok()) }

    fn from_isize(n: isize) -> Option<Self> { T::from_isize(n).and_then(|n| NotNaN::new(n).ok()) }
    fn from_i8(n: i8) -> Option<Self> { T::from_i8(n).and_then(|n| NotNaN::new(n).ok()) }
    fn from_i16(n: i16) -> Option<Self> { T::from_i16(n).and_then(|n| NotNaN::new(n).ok()) }
    fn from_i32(n: i32) -> Option<Self> { T::from_i32(n).and_then(|n| NotNaN::new(n).ok()) }
    fn from_usize(n: usize) -> Option<Self> { T::from_usize(n).and_then(|n| NotNaN::new(n).ok()) }
    fn from_u8(n: u8) -> Option<Self> { T::from_u8(n).and_then(|n| NotNaN::new(n).ok()) }
    fn from_u16(n: u16) -> Option<Self> { T::from_u16(n).and_then(|n| NotNaN::new(n).ok()) }
    fn from_u32(n: u32) -> Option<Self> { T::from_u32(n).and_then(|n| NotNaN::new(n).ok()) }
    fn from_f32(n: f32) -> Option<Self> { T::from_f32(n).and_then(|n| NotNaN::new(n).ok()) }
    fn from_f64(n: f64) -> Option<Self> { T::from_f64(n).and_then(|n| NotNaN::new(n).ok()) }
}

impl<T: Float + ToPrimitive> ToPrimitive for NotNaN<T> {
    fn to_i64(&self) -> Option<i64> { self.0.to_i64() }
    fn to_u64(&self) -> Option<u64> { self.0.to_u64() }

    fn to_isize(&self) -> Option<isize> { self.0.to_isize() }
    fn to_i8(&self) -> Option<i8> { self.0.to_i8() }
    fn to_i16(&self) -> Option<i16> { self.0.to_i16() }
    fn to_i32(&self) -> Option<i32> { self.0.to_i32() }
    fn to_usize(&self) -> Option<usize> { self.0.to_usize() }
    fn to_u8(&self) -> Option<u8> { self.0.to_u8() }
    fn to_u16(&self) -> Option<u16> { self.0.to_u16() }
    fn to_u32(&self) -> Option<u32> { self.0.to_u32() }
    fn to_f32(&self) -> Option<f32> { self.0.to_f32() }
    fn to_f64(&self) -> Option<f64> { self.0.to_f64() }
}

/// An error indicating a parse error from a string for `NotNaN`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ParseNotNaNError<E> {
    /// A plain parse error from the underlying float type.
    ParseFloatError(E),
    /// The parsed float value resulted in a NaN.
    IsNaN,
}

impl<E: fmt::Debug> Error for ParseNotNaNError<E> {
    fn description(&self) -> &str {
        return "Error parsing a not-NaN floating point value";
    }
}

impl<E: fmt::Debug> fmt::Display for ParseNotNaNError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

impl<T: Float + Num> Num for NotNaN<T> {
    type FromStrRadixErr = ParseNotNaNError<T::FromStrRadixErr>;

    fn from_str_radix(src: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        T::from_str_radix(src, radix)
            .map_err(|err| ParseNotNaNError::ParseFloatError(err))
            .and_then(|n| NotNaN::new(n).map_err(|_| ParseNotNaNError::IsNaN))
    }
}

impl<T: Float + Signed> Signed for NotNaN<T> {
    fn abs(&self) -> Self { NotNaN(self.0.abs()) }

    fn abs_sub(&self, other: &Self) -> Self {
        NotNaN::new(self.0.abs_sub(other.0)).expect("Subtraction resulted in NaN")
    }

    fn signum(&self) -> Self { NotNaN(self.0.signum()) }
    fn is_positive(&self) -> bool { self.0.is_positive() }
    fn is_negative(&self) -> bool { self.0.is_negative() }
}

#[cfg(feature = "serde")]
mod impl_serde {
    extern crate serde;
    use self::serde::{Serialize, Serializer, Deserialize, Deserializer};
    use self::serde::de::{Error, Unexpected};
    use super::{OrderedFloat, NotNaN};
    use num_traits::Float;
    use std::f64;

    #[cfg(test)]
    extern crate serde_test;
    #[cfg(test)]
    use self::serde_test::{Token, assert_tokens, assert_de_tokens_error};

    impl<T: Float + Serialize> Serialize for OrderedFloat<T> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            self.0.serialize(s)
        }
    }

    impl<'de, T: Float + Deserialize<'de>> Deserialize<'de> for OrderedFloat<T> {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            T::deserialize(d).map(OrderedFloat)
        }
    }

    impl<T: Float + Serialize> Serialize for NotNaN<T> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            self.0.serialize(s)
        }
    }

    impl<'de, T: Float + Deserialize<'de>> Deserialize<'de> for NotNaN<T> {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let float = T::deserialize(d)?;
            NotNaN::new(float).map_err(|_| {
                Error::invalid_value(Unexpected::Float(f64::NAN), &"float (but not NaN)")
            })
        }
    }

    #[test]
    fn test_ordered_float() {
        let float = OrderedFloat(1.0f64);
        assert_tokens(&float, &[Token::F64(1.0)]);
    }

    #[test]
    fn test_not_nan() {
        let float = NotNaN(1.0f64);
        assert_tokens(&float, &[Token::F64(1.0)]);
    }

    #[test]
    fn test_fail_on_nan() {
        assert_de_tokens_error::<NotNaN<f64>>(
            &[Token::F64(f64::NAN)],
            "invalid value: floating point `NaN`, expected float (but not NaN)");
    }
}
