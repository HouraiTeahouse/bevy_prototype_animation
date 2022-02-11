use crate::{
    curves::{Curve, KeyframeIndex},
    math::interpolation::Lerp,
    Sample,
};
use serde::{Deserialize, Serialize};

/// Curve with evenly spaced keyframes, in another words a curve with a fixed frame rate.
///
/// This curve maintains the faster sampling rate over a wide range of frame rates, because
/// it doesn't rely on keyframe cursor. As a downside, it will have a bigger memory foot print.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct CurveFixed<T>
where
    T: Lerp<Output = T> + Clone,
{
    /// Frames per second
    frame_rate: f32,
    /// Negative number of frames before the curve starts, it's stored
    /// in a `f32` to avoid castings in the when sampling the curve and also
    /// negated to use [`std::f32::mul_add`]
    negative_frame_offset: f32,
    pub keyframes: Vec<T>,
}

impl<T> CurveFixed<T>
where
    T: Lerp<Output = T> + Clone,
{
    pub fn from_keyframes(frame_rate: f32, keyframes: Vec<T>) -> Self {
        Self::from_keyframes_with_offset(frame_rate, 0, keyframes)
    }

    pub fn from_keyframes_with_offset(
        frame_rate: f32,
        frame_offset: i32,
        keyframes: Vec<T>,
    ) -> Self {
        Self {
            frame_rate,
            negative_frame_offset: -(frame_offset as f32),
            keyframes,
        }
    }

    pub fn from_constant(v: T) -> Self {
        Self {
            frame_rate: 30.0,
            negative_frame_offset: 0.0,
            keyframes: vec![v],
        }
    }

    #[inline]
    pub fn frame_rate(&self) -> f32 {
        self.frame_rate
    }

    #[inline]
    pub fn set_frame_rate(&mut self, frame_rate: f32) {
        self.frame_rate = frame_rate;
    }

    /// Sets the start keyframe index.
    ///
    /// Adds a starting delay in multiples of the frame duration `(1 / frame_rate)`
    #[inline]
    pub fn set_frame_offset(&mut self, offset: i32) {
        self.negative_frame_offset = -offset as f32;
    }

    /// Number of the start keyframe
    #[inline]
    pub fn frame_offset(&self) -> i32 {
        -self.negative_frame_offset as i32
    }

    /// `true` when this `CurveFixed` doesn't have any keyframe
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.keyframes.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.keyframes.iter_mut()
    }
}

impl<T> Sample<T> for CurveFixed<T>
where
    T: Lerp<Output = T> + Clone,
{
    fn sample(&self, time: f32) -> T {
        // Make sure to have at least one sample
        assert!(!self.keyframes.is_empty(), "track is empty");

        let frame_time = time * self.frame_rate + self.negative_frame_offset;
        let frame_time = frame_time.clamp(0.0, (self.keyframe_count() - 1) as f32);
        let frame = frame_time.trunc();
        let time = frame_time - frame;
        let frame_idx = frame as usize;
        if frame_idx >= self.keyframe_count() - 1 {
            self.keyframes.last().unwrap().clone()
        } else {
            // Lerp the value
            // SAFE: Both frame_idx and frame_idx + 1 are valid.
            unsafe {
                <&T as Lerp>::lerp_unclamped(
                    self.keyframes.get_unchecked(frame_idx),
                    self.keyframes.get_unchecked(frame_idx + 1),
                    time,
                )
            }
        }
    }
}

impl<T> Curve<T> for CurveFixed<T>
where
    T: Lerp<Output = T> + Clone,
{
    fn duration(&self) -> f32 {
        ((self.keyframe_count() as f32 - 1.0 - self.negative_frame_offset) / self.frame_rate)
            .max(0.0)
    }

    #[inline]
    fn time_offset(&self) -> f32 {
        -self.negative_frame_offset / self.frame_rate
    }

    #[inline]
    fn keyframe_count(&self) -> usize {
        self.keyframes.len()
    }

    #[inline]
    fn sample_with_cursor(&self, _: KeyframeIndex, time: f32) -> (KeyframeIndex, T) {
        (0, self.sample(time))
    }
}
