use std::cmp::Ordering;
use std::marker::PhantomData;

use num_complex::Complex;

use super::{compare_f32, compare_f64, Collate};

/// Compare the `left` and `right` [`Complex`] numbers for collation.
pub fn compare_c32(left: &Complex<f32>, right: &Complex<f32>) -> Ordering {
    compare_f32(&left.norm_sqr(), &right.norm_sqr())
}

/// Compare the `left` and `right` [`Complex`] numbers for collation.
pub fn compare_c64(left: &Complex<f64>, right: &Complex<f64>) -> Ordering {
    compare_f64(&left.norm_sqr(), &right.norm_sqr())
}

/// Implements [`Collate`] for [`Complex`] values.
#[derive(Copy, Clone)]
pub struct ComplexCollator<T> {
    phantom: PhantomData<T>,
}

impl Collate for ComplexCollator<f32> {
    type Value = Complex<f32>;

    fn compare(&self, left: &Self::Value, right: &Self::Value) -> Ordering {
        compare_c32(left, right)
    }
}

impl Default for ComplexCollator<f32> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl Collate for ComplexCollator<f64> {
    type Value = Complex<f64>;

    fn compare(&self, left: &Self::Value, right: &Self::Value) -> Ordering {
        compare_c64(left, right)
    }
}

impl Default for ComplexCollator<f64> {
    fn default() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}
