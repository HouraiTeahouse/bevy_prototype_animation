use crate::util;
use bevy_core::FloatOrd;
use bevy_math::*;
use bevy_reflect::Reflect;
use bevy_transform::prelude::Transform;

pub struct BlendInput<T> {
    pub weight: f32,
    pub value: T,
    pub additive: bool,
}

pub trait Animatable: Reflect + Sized + Send + Sync + 'static {
    fn interpolate(a: &Self, b: &Self, time: f32) -> Self;
    fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self;
}

macro_rules! impl_float_animatable_32 {
    ($ty: ty) => {
        impl Animatable for $ty {
            #[inline(always)]
            fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
                (*a) * (1.0 - t) + (*b) * t
            }

            #[inline(always)]
            fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
                let mut value = Default::default();
                for input in inputs {
                    if input.additive {
                        value += input.weight * input.value;
                    } else {
                        value = Self::interpolate(&value, &input.value, input.weight);
                    }
                }
                value
            }
        }
    };
}

macro_rules! impl_float_animatable_64 {
    ($ty: ty) => {
        impl Animatable for $ty {
            #[inline(always)]
            fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
                let t = f64::from(t);
                (*a) * (1.0 - t) + (*b) * t
            }

            #[inline(always)]
            fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
                let mut value = Default::default();
                for input in inputs {
                    if input.additive {
                        value += f64::from(input.weight) * input.value;
                    } else {
                        value = Self::interpolate(&value, &input.value, input.weight);
                    }
                }
                value
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
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        (*a) * (1.0 - t) + (*b) * t
    }

    #[inline(always)]
    fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
        let mut value = Vec3A::ZERO;
        for input in inputs {
            if input.additive {
                value += input.weight * Vec3A::from(input.value);
            } else {
                value = Vec3A::interpolate(&value, &Vec3A::from(input.value), input.weight);
            }
        }
        Self::from(value)
    }
}

impl Animatable for bool {
    #[inline]
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        util::step_unclamped(*a, *b, t)
    }

    #[inline]
    fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
        inputs
            .max_by(|a, b| FloatOrd(a.weight).cmp(&FloatOrd(b.weight)))
            .map(|input| input.value)
            .unwrap_or(false)
    }
}

impl Animatable for Transform {
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        Self {
            translation: Vec3::interpolate(&a.translation, &b.translation, t),
            rotation: Quat::interpolate(&a.rotation, &b.rotation, t),
            scale: Vec3::interpolate(&a.scale, &b.scale, t),
        }
    }

    fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
        let mut translation = Vec3A::ZERO;
        let mut scale = Vec3A::ZERO;
        let mut rotation = Quat::IDENTITY;

        for input in inputs {
            if input.additive {
                translation += input.weight * Vec3A::from(input.value.translation);
                scale += input.weight * Vec3A::from(input.value.scale);
                rotation = (input.value.rotation * input.weight) * rotation;
            } else {
                translation = Vec3A::interpolate(
                    &translation,
                    &Vec3A::from(input.value.translation),
                    input.weight,
                );
                scale = Vec3A::interpolate(&scale, &Vec3A::from(input.value.scale), input.weight);
                rotation = Quat::interpolate(&rotation, &input.value.rotation, input.weight);
            }
        }

        Self {
            translation: Vec3::from(translation),
            rotation,
            scale: Vec3::from(scale),
        }
    }
}

impl Animatable for Quat {
    /// Performs an nlerp, because it's cheaper and easier to combine with other animations,
    /// reference: http://number-none.com/product/Understanding%20Slerp,%20Then%20Not%20Using%20It/
    #[inline]
    fn interpolate(a: &Self, b: &Self, t: f32) -> Self {
        // Make sure is always the short path, look at this: https://github.com/mgeier/quaternion-nursery
        let b = if a.dot(*b) < 0.0 { -*b } else { *b };

        let a: Vec4 = (*a).into();
        let b: Vec4 = b.into();
        let rot = Vec4::interpolate(&a, &b, t);
        let inv_mag = util::approx_rsqrt(rot.dot(rot));
        Quat::from_vec4(rot * inv_mag)
    }

    #[inline]
    fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
        let mut value = Self::IDENTITY;
        for input in inputs {
            value = Self::interpolate(&value, &input.value, input.weight);
        }
        value
    }
}

// impl<T: Animatable> Animatable for Range<T> {
//     fn interpolate(a: Self, b: Self, t: f32) -> Self {
//         Self {
//             start: <T as Animatable>::interpolate(&a.start, &b.start, t),
//             end: <T as Animatable>::interpolate(&a.end, &b.end, t),
//         }
//     }

//     fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
//         let mut starts = Vec::new();
//         let mut ends = Vec::new();

//         for input in inputs {
//             starts.push(BlendInput {
//                 weight: input.weight,
//                 value: input.value.start,
//             });
//             ends.push(BlendInput {
//                 weight: input.weight,
//                 value: input.value.end,
//             });
//         }

//         Self {
//             start: <T as Animatable>::blend(starts.into_iter()),
//             end: <T as Animatable>::blend(ends.into_iter()),
//         }
//     }
// }

// impl<T: Animatable + Clone> Animatable for RangeInclusive<T> {
//     fn interpolate(a: Self, b: Self, t: f32) -> Self {
//         Self::new(
//             <T as Animatable>::interpolate(a.start(), b.start(), t),
//             <T as Animatable>::interpolate(a.end(), b.end(), t),
//         )
//     }

//     fn blend(inputs: impl Iterator<Item = BlendInput<Self>>) -> Self {
//         let mut starts = Vec::new();
//         let mut ends = Vec::new();

//         for input in inputs {
//             starts.push(BlendInput {
//                 weight: input.weight,
//                 value: input.value.start().clone(),
//             });
//             ends.push(BlendInput {
//                 weight: input.weight,
//                 value: input.value.end().clone(),
//             });
//         }

//         Self::new(
//             <T as Animatable>::blend(starts.into_iter()),
//             <T as Animatable>::blend(ends.into_iter()),
//         )
//     }
// }
