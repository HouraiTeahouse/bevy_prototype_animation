use crate::Animatable;
use bevy_asset::{Asset, Handle};
use thiserror::Error;

pub mod compressed;
mod fixed;
// mod variable;
//mod variable_linear;

pub use fixed::*;
// pub use variable::*;
//pub use variable_linear::*;

// use crate::math::interpolation::Lerp;
use bevy_math::*;

/// Points to a keyframe inside a given curve.
///
/// When sampling curves with variable framerate like [`CurveVariable`] and [`CurveVariableLinear`]
/// is useful to keep track of a particular keyframe near the last sampling time, this keyframe index
/// is referred as cursor and speeds up sampling when the next time is close to the previous on, that
/// happens very often when playing a animation for instance.
///
/// **NOTE** By default each keyframe is indexed using a `u16` to reduce memory usage for the curve cursor cache when implemented
pub type KeyframeIndex = u16;

/// Defines a curve function that can be sampled.
/// Typically composed made of keyframes
pub trait Curve<T>: Send + Sync + 'static {
    /// The total duration of the curve in seconds.
    fn duration(&self) -> f32;

    /// The time offset before the first keyframe.
    fn time_offset(&self) -> f32;

    /// The number of keyframes within the curve.
    fn keyframe_count(&self) -> usize;

    fn sample(&self, time: f32) -> T;

    /// Samples the curve starting from some keyframe cursor, this make the common case `O(1)`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mut time = 0.0;
    /// let mut current_cursor = 0;
    /// loop {
    ///     let (next_cursor, value) = curve.sample_with_cursor(current_cursor, time);
    ///     current_cursor = next_cursor;
    ///     time += 0.01333f;
    ///     /// ...
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics when the curve is empty, e.i. has no keyframes
    fn sample_with_cursor(&self, cursor: KeyframeIndex, time: f32) -> (KeyframeIndex, T);
}

/// Resamples the curve preserving the loop cycle.
///
/// [`CurveFixed`] only supports evenly spaced keyframes, because of that the curve duration
/// is always a multiple of the frame rate. So resampling a curve will always round up their duration
/// but it's still possible to preserve the loop cycle, i.e. both start and end keyframes will be remain the same,
/// which is a very desired property.
pub fn resample_preserving_loop<T: Animatable + Clone>(
    curve: &impl Curve<T>,
    frame_rate: f32,
) -> CurveFixed<T> {
    // get properties
    let offset = curve.time_offset();
    let duration = curve.duration();

    let frame_count = (duration * frame_rate).round() as usize;
    let frame_offset = (offset * frame_rate).round() as i32;

    let normalize = 1.0 / (frame_count - 1) as f32;
    let mut cursor0 = 0;
    let keyframes = (0..frame_count)
        .into_iter()
        .map(|f| {
            let time = duration * (f as f32 * normalize) + offset;
            let (cursor1, value) = curve.sample_with_cursor(cursor0, time);
            cursor0 = cursor1;
            value
        })
        .collect::<Vec<_>>();

    // TODO: copy the start and end keyframes, because f32 precision might not be enough to preserve the loop
    // keyframes[0] = self.value_at(0);
    // keyframes[frame_count - 1] = self.value_at((self.len() - 1) as KeyframeIndex);

    CurveFixed::from_keyframes_with_offset(frame_rate, frame_offset, keyframes)
}

// impl<C: Curve<Handle<T>>, T: Asset> Curve<Handle<T>> for C {
//     fn duration(&self) -> f32 {
//         Self::duration(self)
//     }

//     fn time_offset(&self) -> f32 {
//         <Self as Curve<Handle<T>>>::time_offset(self)
//     }

//     fn keyframe_count(&self) -> usize {
//         <Self as Curve<Handle<T>>>::keyframe_count(self)
//     }

//     fn sample(&self, time: f32) -> Handle<T> {
//         let id = <Self as Curve<Handle<T>>>::sample(self, time);
//         Handle::<T>::weak(id)
//     }

//     fn sample_with_cursor(&self, cursor: KeyframeIndex, time: f32) -> (KeyframeIndex, Handle<T>) {
//         let (cursor, id) = <Self as Curve<Handle<T>>>::sample_with_cursor(self, cursor, time);
//         (cursor, Handle::<T>::weak(id))
//     }
// }

#[derive(Error, Debug)]
pub enum CurveError {
    #[error("number of keyframes time stamps and values doesn't match")]
    MismatchedLength,
    #[error("limit of {0} keyframes exceeded")]
    KeyframeLimitReached(usize),
    #[error("keyframes aren't sorted by time")]
    NotSorted,
}
