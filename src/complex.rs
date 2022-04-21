use std::cmp::Ordering;

use num_complex::Complex;

use super::{Collate, FloatCollator};

pub struct ComplexCollator<T> {
    float: FloatCollator<T>,
}

impl<T> Default for ComplexCollator<T> where FloatCollator<T>: Default {
    fn default() -> Self {
        Self { float: FloatCollator::default() }
    }
}

impl Collate for ComplexCollator<f32> {
    type Value = Complex<f32>;

    fn compare(&self, left: &Self::Value, right: &Self::Value) -> Ordering {
        self.float.compare(&left.norm_sqr(), &right.norm_sqr())
    }
}

impl Collate for ComplexCollator<f64> {
    type Value = Complex<f64>;

    fn compare(&self, left: &Self::Value, right: &Self::Value) -> Ordering {
        self.float.compare(&left.norm_sqr(), &right.norm_sqr())
    }
}
