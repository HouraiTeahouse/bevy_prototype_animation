use bevy_math::*;
use crate::math::interpolation::util;
use std::ops::{Range, RangeInclusive};

/// Trait for computing the [linear interpolation] between two
/// values of a given type.
///
/// This generally only applies to continuous numerical types like
/// `f32` or `f64`. However, additional integral types have this trait
/// implemented for the purposes of stepwise interpolation in animation.
///
/// [linear interpolation]: https://en.wikipedia.org/wiki/Linear_interpolation
pub trait Lerp: Sized {
    /// Linearly interpolates between `a` and `b` using `t` as
    /// a time parameter.
    ///
    /// Generally defined in mathematics as `a * (1.0 - t) + b * t`,
    /// most implementors should follow this definition. Non-continuous
    /// types implementing this trait may return step-wise interpolations.
    ///
    /// This function clamps the provided `t` parameter to a range of `[0, 1]`.
    /// For a unclamped version, use [`lerp_unclamped`] instead.
    ///
    /// [`lerp_unclamped`]: Self::lerp_unclamped
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self::lerp_unclamped(a, b, t.clamp(0.0, 1.0))
    }

    /// Linearly interpolates between `a` and `b` using `t` as
    /// a time parameter.
    ///
    /// Generally defined in mathematics as `a * (1.0 - t) + b * t`,
    /// most implementors should follow this definition. Non-continuous
    /// types implementing this trait may return step-wise interpolations.
    ///
    /// This function does not clamp the provided `t` parameter.
    /// For a clamped version, use [`lerp`] instead.
    ///
    /// [`lerp`]: Self::lerp
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self;
}

macro_rules! impl_continuous_lerp_32 {
    ($ty: ty) => {
        impl Lerp for $ty {
            #[inline]
            fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
                (*a) * (1.0 - t) + (*b) * t
            }
        }
    };
}

macro_rules! impl_continuous_lerp_64 {
    ($ty: ty) => {
        impl Lerp for $ty {
            #[inline]
            fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
                let t = f64::from(t);
                (*a) * (1.0 - t) + (*b) * t
            }
        }
    };
}

impl_continuous_lerp_32!(f32);
impl_continuous_lerp_32!(Vec2);
impl_continuous_lerp_32!(Vec3);
impl_continuous_lerp_32!(Vec3A);
impl_continuous_lerp_32!(Vec4);

impl_continuous_lerp_64!(f64);
impl_continuous_lerp_64!(DVec2);
impl_continuous_lerp_64!(DVec3);
impl_continuous_lerp_64!(DVec4);

impl Lerp for bool {
    #[inline]
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        util::step_unclamped(a, b, t)
    }
}

impl Lerp for Quat {
    /// Performs an nlerp, because it's cheaper and easier to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    #[inline]
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        let mut b = *b;

        // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
        if a.dot(b) < 0.0 {
            b = -b;
        }

        let a: Vec4 = (*a).into();
        let b: Vec4 = b.into();

        let rot = Vec4::lerp_unclamped(&a, &b, t);
        let inv_mag = util::approx_rsqrt(rot.dot(rot));
        Quat::from_vec4(rot * inv_mag)
    }
}

impl<T: Lerp + Clone> Lerp for Option<T> {
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        match (a, b) {
            (Some(a), Some(b)) => Some(T::lerp_unclamped(a, b, t)),
            _ => util::step_unclamped(a, b, t), // change from `Some(T)` to `None` and vice versa
        }
    }
}

impl<T: Lerp + Clone> Lerp for Range<T> {
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        Range {
            start: <T as Lerp>::lerp_unclamped(&a.start, &b.start, t),
            end: <T as Lerp>::lerp_unclamped(&a.end, &b.end, t),
        }
    }
}

impl<T: Lerp + Clone> Lerp for RangeInclusive<T> {
    fn lerp_unclamped(a: &Self, b: &Self, t: f32) -> Self {
        RangeInclusive::new(
            <T as Lerp>::lerp_unclamped(a.start(), b.start(), t),
            <T as Lerp>::lerp_unclamped(a.end(), b.end(), t),
        )
    }
}
