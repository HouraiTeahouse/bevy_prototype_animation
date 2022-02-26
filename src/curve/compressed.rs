use crate::{
    curve::{Curve, CurveFixed, KeyframeIndex},
    Animatable,
};
use bevy_core::FloatOrd;
use bevy_math::*;
use bevy_transform::prelude::Transform;

enum CompressedFloat32Storage {
    Static {
        frames: usize,
        value: f32,
    },
    Quantized {
        frames: Box<[u16]>,
        min_value: f32,
        increment: f32,
    },
}

impl CompressedFloat32Storage {
    pub fn quantize(values: impl Iterator<Item = f32>) -> Self {
        let values: Vec<f32> = values.collect();
        assert!(!values.is_empty());
        let mut min_value = FloatOrd(f32::INFINITY);
        let mut max_value = FloatOrd(f32::NEG_INFINITY);
        for value in values.iter() {
            assert!(!value.is_nan());
            let value = FloatOrd(*value);
            min_value = std::cmp::min(min_value, value);
            max_value = std::cmp::max(max_value, value);
        }

        if min_value == max_value {
            Self::Static {
                frames: values.len(),
                value: min_value.0,
            }
        } else {
            let increment = (max_value.0 - min_value.0) / f32::from(u16::MAX);
            let frames = values
                .into_iter()
                .map(|value| ((value - min_value.0) / increment) as u16)
                .collect();

            Self::Quantized {
                frames,
                min_value: min_value.0,
                increment,
            }
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        match self {
            Self::Static { frames, .. } => *frames,
            Self::Quantized { frames, .. } => frames.len(),
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        false
    }

    #[inline(always)]
    pub fn sample(&self, frame_rate: f32, time: f32, time_offset: f32) -> f32 {
        match self {
            Self::Static { value, .. } => *value,
            Self::Quantized {
                frames,
                min_value,
                increment,
            } => {
                let frame_time = time * frame_rate - time_offset;
                let frame_time = frame_time.clamp(0.0, (frames.len() - 1) as f32);
                let frame = frame_time.trunc();
                let time = frame_time - frame;
                let frame_idx = frame as usize;

                if frame_idx >= frames.len() - 1 {
                    *min_value + f32::from(frames[frames.len() - 1]) * *increment
                } else {
                    let start = *min_value + f32::from(frames[frame_idx]) * *increment;
                    let end = *min_value + f32::from(frames[frame_idx + 1]) * *increment;
                    // Interpolate the value
                    f32::interpolate(&start, &end, time)
                }
            }
        }
    }
}

pub struct CompressedFloat32Curve {
    frame_rate: f32,
    time_offset: f32,
    values: CompressedFloat32Storage,
}

impl CompressedFloat32Curve {
    pub fn quantize(src: CurveFixed<f32>) -> Self {
        Self {
            frame_rate: src.frame_rate(),
            time_offset: src.time_offset(),
            values: CompressedFloat32Storage::quantize(src.keyframes.into_iter()),
        }
    }
}

impl Curve<f32> for CompressedFloat32Curve {
    fn duration(&self) -> f32 {
        self.values.len() as f32 * self.frame_rate
    }

    fn time_offset(&self) -> f32 {
        self.time_offset
    }

    fn keyframe_count(&self) -> usize {
        self.values.len()
    }

    fn sample(&self, time: f32) -> f32 {
        self.values.sample(self.frame_rate, time, self.time_offset)
    }

    fn sample_with_cursor(&self, _: KeyframeIndex, time: f32) -> (KeyframeIndex, f32) {
        (0, self.sample(time))
    }
}

pub struct CompressedFloat32x2Curve {
    frame_rate: f32,
    time_offset: f32,
    x: CompressedFloat32Storage,
    y: CompressedFloat32Storage,
}

impl CompressedFloat32x2Curve {
    pub fn quantize(src: CurveFixed<Vec2>) -> Self {
        let x = src.keyframes.iter().map(|vec| vec.x);
        let y = src.keyframes.iter().map(|vec| vec.y);
        Self {
            frame_rate: src.frame_rate(),
            time_offset: src.time_offset(),
            x: CompressedFloat32Storage::quantize(x),
            y: CompressedFloat32Storage::quantize(y),
        }
    }
}

impl Curve<Vec2> for CompressedFloat32x2Curve {
    fn duration(&self) -> f32 {
        self.x.len() as f32 * self.frame_rate
    }

    fn time_offset(&self) -> f32 {
        self.time_offset
    }

    fn keyframe_count(&self) -> usize {
        self.x.len()
    }

    fn sample(&self, time: f32) -> Vec2 {
        let x = self.x.sample(self.frame_rate, time, self.time_offset);
        let y = self.y.sample(self.frame_rate, time, self.time_offset);
        Vec2::new(x, y)
    }

    fn sample_with_cursor(&self, _: KeyframeIndex, time: f32) -> (KeyframeIndex, Vec2) {
        (0, self.sample(time))
    }
}

pub struct CompressedFloat32x3Curve {
    frame_rate: f32,
    time_offset: f32,
    x: CompressedFloat32Storage,
    y: CompressedFloat32Storage,
    z: CompressedFloat32Storage,
}

impl CompressedFloat32x3Curve {
    pub fn quantize(src: CurveFixed<Vec3>) -> Self {
        let x = src.keyframes.iter().map(|vec| vec.x);
        let y = src.keyframes.iter().map(|vec| vec.y);
        let z = src.keyframes.iter().map(|vec| vec.z);
        Self {
            frame_rate: src.frame_rate(),
            time_offset: src.time_offset(),
            x: CompressedFloat32Storage::quantize(x),
            y: CompressedFloat32Storage::quantize(y),
            z: CompressedFloat32Storage::quantize(z),
        }
    }
}

impl Curve<Vec3> for CompressedFloat32x3Curve {
    fn duration(&self) -> f32 {
        self.x.len() as f32 * self.frame_rate
    }

    fn time_offset(&self) -> f32 {
        self.time_offset
    }

    fn keyframe_count(&self) -> usize {
        self.x.len()
    }

    fn sample(&self, time: f32) -> Vec3 {
        let x = self.x.sample(self.frame_rate, time, self.time_offset);
        let y = self.y.sample(self.frame_rate, time, self.time_offset);
        let z = self.z.sample(self.frame_rate, time, self.time_offset);
        Vec3::new(x, y, z)
    }

    fn sample_with_cursor(&self, _: KeyframeIndex, time: f32) -> (KeyframeIndex, Vec3) {
        (0, self.sample(time))
    }
}

impl Curve<Vec3A> for CompressedFloat32x3Curve {
    fn duration(&self) -> f32 {
        self.x.len() as f32 * self.frame_rate
    }

    fn time_offset(&self) -> f32 {
        self.time_offset
    }

    fn keyframe_count(&self) -> usize {
        self.x.len()
    }

    fn sample(&self, time: f32) -> Vec3A {
        let x = self.x.sample(self.frame_rate, time, self.time_offset);
        let y = self.y.sample(self.frame_rate, time, self.time_offset);
        let z = self.z.sample(self.frame_rate, time, self.time_offset);
        Vec3A::new(x, y, z)
    }

    fn sample_with_cursor(&self, _: KeyframeIndex, time: f32) -> (KeyframeIndex, Vec3A) {
        (0, self.sample(time))
    }
}

pub struct CompressedFloat32x4Curve {
    frame_rate: f32,
    time_offset: f32,
    x: CompressedFloat32Storage,
    y: CompressedFloat32Storage,
    z: CompressedFloat32Storage,
    w: CompressedFloat32Storage,
}

impl Curve<Vec4> for CompressedFloat32x4Curve {
    fn duration(&self) -> f32 {
        self.x.len() as f32 * self.frame_rate
    }

    fn time_offset(&self) -> f32 {
        self.time_offset
    }

    fn keyframe_count(&self) -> usize {
        self.x.len()
    }

    fn sample(&self, time: f32) -> Vec4 {
        let x = self.x.sample(self.frame_rate, time, self.time_offset);
        let y = self.y.sample(self.frame_rate, time, self.time_offset);
        let z = self.z.sample(self.frame_rate, time, self.time_offset);
        let w = self.w.sample(self.frame_rate, time, self.time_offset);
        Vec4::new(x, y, z, w)
    }

    fn sample_with_cursor(&self, _: KeyframeIndex, time: f32) -> (KeyframeIndex, Vec4) {
        (0, self.sample(time))
    }
}

impl CompressedFloat32x4Curve {
    pub fn quantize(src: CurveFixed<Vec3>) -> Self {
        let x = src.keyframes.iter().map(|vec| vec.x);
        let y = src.keyframes.iter().map(|vec| vec.y);
        let z = src.keyframes.iter().map(|vec| vec.z);
        let w = src.keyframes.iter().map(|vec| vec.z);
        Self {
            frame_rate: src.frame_rate(),
            time_offset: src.time_offset(),
            x: CompressedFloat32Storage::quantize(x),
            y: CompressedFloat32Storage::quantize(y),
            z: CompressedFloat32Storage::quantize(z),
            w: CompressedFloat32Storage::quantize(w),
        }
    }
}

pub struct CompressedTransformCurve {
    frame_rate: f32,
    time_offset: f32,

    translation_x: CompressedFloat32Storage,
    translation_y: CompressedFloat32Storage,
    translation_z: CompressedFloat32Storage,

    scale_x: CompressedFloat32Storage,
    scale_y: CompressedFloat32Storage,
    scale_z: CompressedFloat32Storage,

    rotation_x: CompressedFloat32Storage,
    rotation_y: CompressedFloat32Storage,
    rotation_z: CompressedFloat32Storage,
    rotation_w: CompressedFloat32Storage,
}
