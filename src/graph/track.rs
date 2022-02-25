use crate::{
    curves::Curve,
    graph::GraphState,
    Animatable, BlendInput,
};
use bevy_reflect::Reflect;
use bevy_utils::HashMap;
use std::{
    any::{Any, TypeId},
    borrow::Cow,
    sync::Arc,
};

pub(super) struct GraphClips {
    tracks: HashMap<Cow<'static, str>, Box<dyn Track + 'static>>,
}

impl GraphClips {
    pub(super) fn sample_property(
        &self,
        key: impl Into<Cow<'static, str>>,
        state: &GraphState,
        output: &mut dyn Reflect,
    ) -> Result<(), TrackError> {
        let key = key.into();
        let track = self.tracks.get(&key).ok_or(TrackError::MissingTrack)?;
        track.blend(state, output)
    }
}

pub enum TrackError {
    IncorrectType,
    MissingTrack,
}

/// A non-generic interface for all [`Track<T>`] that can be used to hide
/// the internal type-specific implementation.
trait Track: Any {
    fn as_mut_any(&mut self) -> &mut dyn Any;
    // fn blend<T: Animatable>(&self, state: &GraphState) -> Result<T, TrackError>;
    fn blend(&self, state: &GraphState, output: &mut dyn Reflect) -> Result<(), TrackError>;
}

impl dyn Track {
    pub fn add_curve<C: Animatable>(&mut self, clip_id: ClipId, curve: Arc<dyn Curve<C>>) -> Result<(), TrackError> {
        match self.as_mut_any().downcast_mut::<CurveTrack<C>>() {
            Some(track) => Ok(track.add_curve(clip_id, curve)),
            None => Err(TrackError::IncorrectType)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipId(pub u16);

pub(crate) struct CurveTrack<T: Animatable> {
    curves: Vec<Option<Arc<dyn Curve<T>>>>,
}

impl<T: Animatable> CurveTrack<T> {
    fn add_curve(&mut self, clip_id: ClipId, curve: Arc<dyn Curve<T>>) {
        let idx = clip_id.0 as usize;

        // I assume this was your intent with `track.curves.fill_with(idx, || None);`
        let new_idxs = idx.saturating_sub(self.curves.len());
        self.curves.extend(std::iter::repeat(None).take(new_idxs));

        self.curves[idx] = Some(curve);
    }

    fn sample_and_blend(&self, state: &GraphState) -> T {
        let inputs = state
            .clips
            .iter()
            .zip(self.curves.iter())
            .filter(|(clip, curve)| clip.weight != 0.0 && curve.is_some())
            .map(|(clip, curve)| BlendInput {
                weight: clip.weight,
                value: curve.as_ref().unwrap().sample(clip.time),
            });

        T::blend(inputs)
    }
}

impl<T: Animatable> Track for CurveTrack<T> {
    fn as_mut_any(&mut self) -> &mut dyn Any { self as &mut _ }
    fn blend(&self, state: &GraphState, output: &mut dyn Reflect) -> Result<(), TrackError> {
        if output.any().type_id() == TypeId::of::<T>() {
            output.apply(&self.sample_and_blend(state));
            Ok(())
        } else {
            Err(TrackError::IncorrectType)
        }
    }
}