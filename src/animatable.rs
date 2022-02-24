use crate::math::interpolation::util;
use bevy_math::*;
use bevy_reflect::Reflect;
use std::ops::{Range, RangeInclusive};

pub struct BlendInput<T> {
    pub weight: f32,
    pub value: T,
}

pub trait Animatable: Reflect + Sized {
    fn interpolate(a: &Self, b: &Self, time: f32) -> Self;
    fn blend(inputs: impl Iterator<Item=BlendInput<Self>) -> Self;
}

macro_rules! impl_float_animatable_32 {
    ($ty: ty) => {
        impl Animatable for $ty {
            #[inline(always)]
            fn interpolate(a: Self, b: Self, t: f32) -> Self::Output {
                *a * (1.0 - t) + *b * t
            }

            #[inline(always)]
            fn blend(inputs: impl Iterator<Item=BlendInput<Self>>) -> Self {
                inputs
                    .map(|input| input.weight * input.value)
                    .sum()
            }
        }
    };
}

macro_rules! impl_float_animatable_64 {
    ($ty: ty) => {
        impl Animatable for $ty {
            #[inline(always)]
            fn interpolate(a: Self, b: Self, t: f32) -> Self::Output {
                let t = f64::from(t);
                *a * (1.0 - t) + *b * t
            }

            #[inline(always)]
            fn blend(inputs: impl Iterator<Item=BlendInput<Self>>) -> Self {
                inputs
                    .map(|input| input.weight * input.value)
                    .sum()
            }
        }
    };
}

impl_float_animatable_32!(f32);
impl_float_animatable_32!(Vec2);
impl_float_animatable_32!(Vec3A);
impl_float_animatable_32!(Vec4);

impl_float_animatable_64!(f64);
impl_float_animatable_64!(DVec2);
impl_float_animatable_64!(DVec3);
impl_float_animatable_64!(DVec4);

/// Vec3 is special cased to use Vec3A internally for blending
impl Animatable for Vec3 {
    #[inline(always)]
    fn interpolate(a: Self, b: Self, t: f32) -> Self::Output {
        let t = f64::from(t);
        *a * (1.0 - t) + *b * t
    }

    #[inline(always)]
    fn blend(inputs: impl Iterator<Item=BlendInput<Self>>) -> Self {
        Self::from(
        inputs
            .map(|input| input.weight * Vec3A::from(input.value))
            .sum())
    }
}

impl Lerp for bool {
    type Output = Self;

    #[inline]
    fn lerp_unclamped(a: Self, b: Self, t: f32) -> Self {
        util::step_unclamped(a, b, t)
    }
}

impl Lerp for Quat {
    /// Performs an nlerp, because it's cheaper and easier to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    #[inline]
    fn interpolate(a: Self, mut b: Self, t: f32) -> Self {
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

impl<T: Animatable> Animatable for Range<T> {
    fn interpolate(a: Self, b: Self, t: f32) -> Self {
        Self {
            start: <T as Animatable>::interpolate(&a.start, &b.start, t),
            end: <T as Animatable>::interpolate(&a.end, &b.end, t),
        }
    }

    fn blend(inputs: impl Iterator<Item=BlendInput<Self>>) -> Self {
        let mut starts = Vec::new();
        let mut ends = Vec::new();

        for input in inputs {
            starts.push(BlendInput {
                weight: input.weight,
                value: input.value.start,
            });
            ends.push(BlendInput {
                weight: input.weight,
                value: input.value.end,
            });
        }

        Self {
            start: <T as Animatable>::blend(starts.into_iter()),
            end: <T as Animatable>::blend(ends.into_iter()),
        }
    }
}

impl<T: Animatable + Clone> Animatable for RangeInclusive<T> {
    fn interpolate(a: Self, b: Self, t: f32) -> Self {
        Self::new(
            <T as Animatable>::interpolate(a.start(), b.start(), t),
            <T as Animatable>::interpolate(a.end(), b.end(), t),
        )
    }

    fn blend(inputs: impl Iterator<Item=BlendInput<Self>>) -> Self {
        let mut starts = Vec::new();
        let mut ends = Vec::new();

        for input in inputs {
            starts.push(BlendInput {
                weight: input.weight,
                value: input.value.start().clone(),
            });
            ends.push(BlendInput {
                weight: input.weight,
                value input.value.end().clone(),
            });
        }

        Self::new(
            <T as Animatable>::blend(starts.into_iter()),
            <T as Animatable>::blend(ends.into_iter())
        )
    }
}
