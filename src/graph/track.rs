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
    // fn downcast_mut<T: Animatable>(&mut self) -> Option<&mut CurveTrack<T>>;
    // fn add_curve<T: Animatable>(&mut self, clip_id: ClipId, curve: Arc<dyn Curve<T>>) -> Result<(), TrackError>;
    // fn blend<T: Animatable>(&self, state: &GraphState) -> Result<T, TrackError>;
    fn blend(&self, state: &GraphState, output: &mut dyn Reflect) -> Result<(), TrackError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClipId(pub u16);

pub(crate) struct CurveTrack<T: Animatable> {
    curves: Vec<Option<Arc<dyn Curve<T>>>>,
}

impl<T: Animatable> CurveTrack<T> {
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
    // fn downcast_mut<C: Animatable>(&mut self) -> Option<&mut CurveTrack<C>> {
    //     (self as &mut dyn Any).downcast_mut::<CurveTrack<C>>()
    // }

    // fn add_curve<C: Animatable>(
    //     &mut self,
    //     clip_id: ClipId,
    //     curve: Arc<dyn Curve<C>>
    // ) -> Result<(), TrackError> {
    //     match self.downcast_mut::<C>() {
    //         Some(track) => {
    //             let idx = clip_id.0 as usize;
    //             track.curves.fill_with(idx, || None);
    //             track.curves[idx] = Some(curve);
    //             Ok(())
    //         }
    //         None => Err(TrackError::IncorrectType),
    //     }
    // }

    fn blend(&self, state: &GraphState, output: &mut dyn Reflect) -> Result<(), TrackError> {
        if output.any().type_id() == TypeId::of::<T>() {
            output.apply(&self.sample_and_blend(state));
            Ok(())
        } else {
            Err(TrackError::IncorrectType)
        }
    }
}
