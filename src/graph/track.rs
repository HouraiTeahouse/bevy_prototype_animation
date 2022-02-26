use crate::{
    clip::AnimationClip,
    clip::{ClipCurve, CurveWrapper},
    curve::Curve,
    graph::GraphState,
    path::PropertyPath,
    Animatable, BlendInput,
};
use bevy_reflect::Reflect;
use bevy_utils::{Hashed, PreHashMap};
use std::{
    any::{Any, TypeId},
    sync::Arc,
};

pub(super) struct GraphClips {
    tracks: PreHashMap<PropertyPath, Box<dyn Track + 'static>>,
}

impl GraphClips {
    pub(super) fn properties(&self) -> impl Iterator<Item = &Hashed<PropertyPath>> {
        self.tracks.keys()
    }

    pub(super) fn add_clip(
        &mut self,
        clip_id: ClipId,
        clip: &AnimationClip,
    ) -> Result<(), TrackError> {
        // Verify that the types for each of the tracks are identical before adding any of the curves in.
        for (property, curve) in clip.curves.iter() {
            if let Some(track) = self.tracks.get_mut(property) {
                if curve.value_type_id() != track.value_type_id() {
                    return Err(TrackError::IncorrectType);
                }
            }
        }

        for (property, curve) in clip.curves.iter() {
            if let Some(track) = self.tracks.get_mut(property) {
                track.add_generic_curve(clip_id, curve.as_ref()).unwrap();
            } else {
                self.tracks
                    .insert(property.clone(), curve.into_track(clip_id));
            }
        }
        Ok(())
    }

    pub(super) fn sample<T: Animatable>(
        &self,
        key: &Hashed<PropertyPath>,
        state: &GraphState,
    ) -> Result<T, TrackError> {
        let track = self.tracks.get(key).ok_or(TrackError::MissingTrack)?;
        track.blend(state)
    }

    pub(super) fn sample_property(
        &self,
        key: &Hashed<PropertyPath>,
        state: &GraphState,
        output: &mut dyn Reflect,
    ) -> Result<(), TrackError> {
        let key = key.into();
        let track = self.tracks.get(key).ok_or(TrackError::MissingTrack)?;
        track.blend_via_reflect(state, output)
    }
}

#[derive(Debug)]
pub enum TrackError {
    IncorrectType,
    MissingTrack,
}

/// A non-generic interface for all [`Track<T>`] that can be used to hide
/// the internal type-specific implementation.
pub(crate) trait Track: Any + Send + Sync + 'static {
    fn value_type_id(&self) -> TypeId;
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
    fn add_generic_curve(
        &mut self,
        clip_id: ClipId,
        curve: &dyn ClipCurve,
    ) -> Result<(), TrackError>;
    fn blend_via_reflect(
        &self,
        state: &GraphState,
        output: &mut dyn Reflect,
    ) -> Result<(), TrackError>;
}

impl dyn Track {
    pub(crate) fn blend<T: Animatable>(&self, state: &GraphState) -> Result<T, TrackError> {
        match self.as_any().downcast_ref::<CurveTrack<T>>() {
            Some(track) => Ok(track.sample_and_blend(state)),
            None => Err(TrackError::IncorrectType),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipId(pub u16);

pub(crate) struct CurveTrack<T: Animatable> {
    curves: Vec<Option<Arc<dyn Curve<T>>>>,
}

impl<T: Animatable> CurveTrack<T> {
    pub(crate) fn new(curve: Arc<dyn Curve<T>>, clip_id: ClipId) -> Self {
        let index = clip_id.0 as usize;
        let mut curves = Vec::with_capacity(index);
        curves.resize_with(index + 1, || None);
        curves[index] = Some(curve);
        Self { curves }
    }

    pub(crate) fn add_curve(&mut self, clip_id: ClipId, curve: Arc<dyn Curve<T>>) {
        let idx = clip_id.0 as usize;
        if idx >= self.curves.len() {
            self.curves.resize_with(idx + 1, || None);
        }
        self.curves[idx] = Some(curve);
    }

    pub(crate) fn sample_and_blend(&self, state: &GraphState) -> T {
        let inputs = state
            .clips
            .iter()
            .zip(self.curves.iter())
            .filter(|(clip, curve)| clip.weight != 0.0 && curve.is_some())
            .map(|(clip, curve)| BlendInput {
                weight: clip.weight,
                value: curve.as_ref().unwrap().sample(clip.time),
                // TODO: Expose this at the node level
                additive: false,
            });

        T::blend(inputs)
    }
}

impl<T: Animatable> Track for CurveTrack<T> {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    fn as_any(&self) -> &dyn Any {
        self as &_
    }
    fn as_mut_any(&mut self) -> &mut dyn Any {
        self as &mut _
    }

    fn add_generic_curve(
        &mut self,
        clip_id: ClipId,
        curve: &dyn ClipCurve,
    ) -> Result<(), TrackError> {
        match curve.as_any().downcast_ref::<CurveWrapper<T>>() {
            Some(curve) => Ok(self.add_curve(clip_id, curve.0.clone())),
            None => Err(TrackError::IncorrectType),
        }
    }

    fn blend_via_reflect(
        &self,
        state: &GraphState,
        output: &mut dyn Reflect,
    ) -> Result<(), TrackError> {
        if output.any().type_id() == TypeId::of::<T>() {
            output.apply(&self.sample_and_blend(state));
            Ok(())
        } else {
            Err(TrackError::IncorrectType)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy_math::*;

    assert_impl_all!(GraphClips: Send, Sync);
    assert_impl_all!(TrackError: Send, Sync);
    assert_impl_all!(dyn Track: Send, Sync);
}
