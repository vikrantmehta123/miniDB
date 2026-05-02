//! Max aggregate over integer types.
//!
//! State is `Option<T>`: `None` means no values seen yet (matches SQL's
//! "MAX over zero rows is NULL" behavior).
//!
//! Bound is `Ord` (total order), which admits all integer types but not
//! floats — floats only implement `PartialOrd` because of NaN.

use std::marker::PhantomData;

pub struct Max<T>(PhantomData<T>);

impl<T> Max<T>
where
    T: Copy + Ord,
{
    pub fn init() -> Option<T> {
        None
    }

    pub fn update(state: &mut Option<T>, input: &[T]) {
        let chunk_max = input.iter().copied().max();
        Self::merge(state, chunk_max);
    }

    pub fn merge(a: &mut Option<T>, b: Option<T>) {
        match (*a, b) {
            (_, None) => {}
            (None, Some(bv)) => *a = Some(bv),
            (Some(av), Some(bv)) => *a = Some(av.max(bv)),
        }
    }

    pub fn finalize(state: Option<T>) -> Option<T> {
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_i64_basic() {
        let mut s = Max::<i64>::init();
        Max::<i64>::update(&mut s, &[3, 1, 4, 1, 5, 9, 2, 6]);
        assert_eq!(Max::<i64>::finalize(s), Some(9));
    }

    #[test]
    fn max_i32_negatives() {
        let mut s = Max::<i32>::init();
        Max::<i32>::update(&mut s, &[-10, -3, -7, -1, -50]);
        assert_eq!(Max::<i32>::finalize(s), Some(-1));
    }

    #[test]
    fn max_u8() {
        let mut s = Max::<u8>::init();
        Max::<u8>::update(&mut s, &[10, 200, 50]);
        assert_eq!(Max::<u8>::finalize(s), Some(200));
    }

    #[test]
    fn max_empty_is_none() {
        let mut s = Max::<i64>::init();
        Max::<i64>::update(&mut s, &[]);
        assert_eq!(Max::<i64>::finalize(s), None);
    }

    #[test]
    fn max_merge_matches_full() {
        let data: Vec<i64> = vec![5, 3, 8, 1, 9, 2, 7, 4, 6, 0];

        let full = {
            let mut s = Max::<i64>::init();
            Max::<i64>::update(&mut s, &data);
            Max::<i64>::finalize(s)
        };

        for split in [0, 1, 5, 9, 10] {
            let mut left = Max::<i64>::init();
            Max::<i64>::update(&mut left, &data[..split]);
            let mut right = Max::<i64>::init();
            Max::<i64>::update(&mut right, &data[split..]);
            Max::<i64>::merge(&mut left, right);
            assert_eq!(Max::<i64>::finalize(left), full, "split {split}");
        }
    }

    #[test]
    fn max_merge_with_empty_partial() {
        let mut left = Max::<i32>::init();
        Max::<i32>::update(&mut left, &[1, 2, 3]);
        let right = Max::<i32>::init(); // stays None
        Max::<i32>::merge(&mut left, right);
        assert_eq!(Max::<i32>::finalize(left), Some(3));
    }
}

//! Max aggregate over float types.
//!
//! NaN handling: comparisons against NaN return false in both directions,
//! so NaN values are never adopted as the max and never replace the
//! current max. Effectively, NaNs are skipped. This matches most databases.

pub struct MaxFloat<T>(PhantomData<T>);

impl<T> MaxFloat<T>
where
    T: Copy + PartialOrd,
{
    pub fn init() -> Option<T> {
        None
    }

    pub fn update(state: &mut Option<T>, input: &[T]) {
        for &v in input {
            match *state {
                None => *state = Some(v),
                Some(current) if v > current => *state = Some(v),
                _ => {}
            }
        }
    }

    pub fn merge(a: &mut Option<T>, b: Option<T>) {
        match (*a, b) {
            (_, None) => {}
            (None, Some(bv)) => *a = Some(bv),
            (Some(av), Some(bv)) if bv > av => *a = Some(bv),
            _ => {}
        }
    }

    pub fn finalize(state: Option<T>) -> Option<T> {
        state
    }
}
