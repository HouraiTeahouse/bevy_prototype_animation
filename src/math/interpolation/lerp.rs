use crate::math::interpolation::util;
use bevy_math::*;
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
    type Output;

    /// Linearly interpolates between `a` and `b` using `t` as
    /// a time parameter.
    ///
    /// Generally defined in mathematics as `a + t * (b - a)`,
    /// most implementors should follow this definition. Non-continuous
    /// types implementing this trait may return step-wise interpolations.
    ///
    /// This function clamps the provided `t` parameter to a range of `[0, 1]`.
    /// For a unclamped version, use [`lerp_unclamped`] instead.
    ///
    /// [`lerp_unclamped`]: Self::lerp_unclamped
    fn lerp(a: Self, b: Self, t: f32) -> Self::Output {
        Self::lerp_unclamped(a, b, t.clamp(0.0, 1.0))
    }

    /// Linearly interpolates between `a` and `b` using `t` as
    /// a time parameter.
    ///
    /// Generally defined in mathematics as `a + t * (b - a)`,
    /// most implementors should follow this definition. Non-continuous
    /// types implementing this trait may return step-wise interpolations.
    ///
    /// This function does not clamp the provided `t` parameter.
    /// For a clamped version, use [`lerp`] instead.
    ///
    /// [`lerp`]: Self::lerp
    fn lerp_unclamped(a: Self, b: Self, t: f32) -> Self::Output;
}

macro_rules! impl_continuous_lerp_32 {
    ($ty: ty) => {
        impl Lerp for $ty {
            type Output = Self;

            #[inline(always)]
            fn lerp_unclamped(a: Self, b: Self, t: f32) -> Self::Output {
                a + t * (b - a)
            }
        }
    };
}

macro_rules! impl_continuous_lerp_64 {
    ($ty: ty) => {
        impl Lerp for $ty {
            type Output = Self;

            #[inline(always)]
            fn lerp_unclamped(a: Self, b: Self, t: f32) -> Self::Output {
                a + f64::from(t) * (b - a)
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
    type Output = Self;

    #[inline]
    fn lerp_unclamped(a: Self, b: Self, t: f32) -> Self {
        util::step_unclamped(a, b, t)
    }
}

impl Lerp for Quat {
    type Output = Self;

    /// Performs an nlerp, because it's cheaper and easier to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    #[inline]
    fn lerp_unclamped(a: Self, mut b: Self, t: f32) -> Self {
        // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
        if a.dot(b) < 0.0 {
            b = -b;
        }

        let a: Vec4 = a.into();
        let b: Vec4 = b.into();

        let rot = Vec4::lerp_unclamped(a, b, t);
        let inv_mag = util::approx_rsqrt(rot.dot(rot));
        Quat::from_vec4(rot * inv_mag)
    }
}

impl<T: Lerp + Clone> Lerp for &T {
    type Output = T::Output;

    #[inline(always)]
    fn lerp_unclamped(a: Self, b: Self, t: f32) -> Self::Output {
        <T as Lerp>::lerp_unclamped(a.clone(), b.clone(), t)
    }
}

impl<T: Lerp + Clone> Lerp for &mut T {
    type Output = T::Output;

    #[inline(always)]
    fn lerp_unclamped(a: Self, b: Self, t: f32) -> Self::Output {
        <T as Lerp>::lerp_unclamped(a.clone(), b.clone(), t)
    }
}

impl<T: Lerp<Output = T>> Lerp for Option<T> {
    type Output = Self;
    fn lerp_unclamped(a: Self, b: Self, t: f32) -> Self {
        match (a, b) {
            (Some(a), Some(b)) => Some(T::lerp_unclamped(a, b, t)),
            (a, b) => util::step_unclamped(a, b, t), // change from `Some(T)` to `None` and vice versa
        }
    }
}

impl<T: Lerp<Output = T>> Lerp for Range<T> {
    type Output = Self;
    fn lerp_unclamped(a: Self, b: Self, t: f32) -> Self {
        Self {
            start: <T as Lerp>::lerp_unclamped(a.start, b.start, t),
            end: <T as Lerp>::lerp_unclamped(a.end, b.end, t),
        }
    }
}

impl<T: Lerp<Output = T> + Clone> Lerp for RangeInclusive<T> {
    type Output = Self;
    fn lerp_unclamped(a: Self, b: Self, t: f32) -> Self {
        Self::new(
            <&T as Lerp>::lerp_unclamped(a.start(), b.start(), t),
            <&T as Lerp>::lerp_unclamped(a.end(), b.end(), t),
        )
    }
}
