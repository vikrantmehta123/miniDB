//! Sum aggregate over numeric types.
//!
//! State and output share the input type T. Overflow uses default `+`
//! (panic in debug, wrap in release). A future enhancement could promote
//! narrow integer inputs to a wider accumulator.

use std::marker::PhantomData;
use std::ops::AddAssign;

pub struct Sum<T>(PhantomData<T>);

impl<T> Sum<T>
where
    T: Copy + Default + AddAssign + std::iter::Sum,
{
    pub fn init() -> T {
        T::default()
    }

    pub fn update(state: &mut T, input: &[T]) {
        *state += input.iter().copied().sum::<T>();
    }

    pub fn merge(a: &mut T, b: T) {
        *a += b;
    }

    pub fn finalize(state: T) -> T {
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_i64_single_chunk() {
        let mut state = Sum::<i64>::init();
        Sum::<i64>::update(&mut state, &[1, 2, 3, 4]);
        assert_eq!(Sum::<i64>::finalize(state), 10);
    }

    #[test]
    fn sum_i32_single_chunk() {
        let mut state = Sum::<i32>::init();
        Sum::<i32>::update(&mut state, &[1, 2, 3, 4]);
        assert_eq!(Sum::<i32>::finalize(state), 10);
    }

    #[test]
    fn sum_empty_is_zero() {
        let mut state = Sum::<i64>::init();
        Sum::<i64>::update(&mut state, &[]);
        assert_eq!(Sum::<i64>::finalize(state), 0);
    }

    #[test]
    fn sum_i64_merge_matches_full() {
        let data: Vec<i64> = (1..=100).collect();

        let full = {
            let mut s = Sum::<i64>::init();
            Sum::<i64>::update(&mut s, &data);
            Sum::<i64>::finalize(s)
        };

        for split in [0, 1, 50, 99, 100] {
            let mut left = Sum::<i64>::init();
            Sum::<i64>::update(&mut left, &data[..split]);
            let mut right = Sum::<i64>::init();
            Sum::<i64>::update(&mut right, &data[split..]);
            Sum::<i64>::merge(&mut left, right);
            assert_eq!(Sum::<i64>::finalize(left), full, "split {split}");
        }
    }

    #[test]
    fn sum_i32_merge_matches_full() {
        let data: Vec<i32> = (1..=100).collect();

        let full = {
            let mut s = Sum::<i32>::init();
            Sum::<i32>::update(&mut s, &data);
            Sum::<i32>::finalize(s)
        };

        for split in [0, 1, 50, 99, 100] {
            let mut left = Sum::<i32>::init();
            Sum::<i32>::update(&mut left, &data[..split]);
            let mut right = Sum::<i32>::init();
            Sum::<i32>::update(&mut right, &data[split..]);
            Sum::<i32>::merge(&mut left, right);
            assert_eq!(Sum::<i32>::finalize(left), full, "split {split}");
        }
    }
}
