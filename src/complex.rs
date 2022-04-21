use std::cmp::Ordering;

use num_complex::Complex;

use super::Collate;

impl Collate for Complex<f32> {
    type Value = Complex<f32>;

    fn compare(&self, left: &Self::Value, right: &Self::Value) -> Ordering {
        self.float.compare(&left.norm_sqr(), &right.norm_sqr())
    }
}

impl Collate for Complex<f64> {
    type Value = Complex<f64>;

    fn compare(&self, left: &Self::Value, right: &Self::Value) -> Ordering {
        self.float.compare(&left.norm_sqr(), &right.norm_sqr())
    }
}
