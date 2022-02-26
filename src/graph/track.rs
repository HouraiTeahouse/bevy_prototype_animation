use crate::{
    clip::AnimationClip,
    clip::{ClipCurve, CurveWrapper},
    curve::Curve,
    graph::GraphState,
    path::{EntityPath, FieldPath, PropertyPath},
    Animatable, BlendInput,
};
use bevy_ecs::prelude::Entity;
use bevy_reflect::Reflect;
use bevy_utils::{HashMap, Hashed, PreHashMap};
use std::{
    any::{Any, TypeId},
    sync::Arc,
};

#[derive(Debug, Clone, Copy)]
pub struct BoneId(usize);

pub struct Bone {
    id: BoneId,
    entity: Option<Entity>,
    tracks: PreHashMap<FieldPath, Box<dyn Track + 'static>>,
}

impl Bone {
    pub fn id(&self) -> BoneId {
        self.id
    }

    pub fn properties(&self) -> impl Iterator<Item = &Hashed<FieldPath>> {
        self.tracks.keys()
    }

    /// Gets the currently bound entity.
    ///
    /// This may not be a valid entity ID even if available.
    pub fn entity(&self) -> Option<Entity> {
        self.entity
    }

    pub(crate) fn set_entity(&mut self, entity: Option<Entity>) {
        self.entity = entity;
    }

    // TODO: Find a way to expose this without exposing internal state.
    pub(crate) fn sample<T: Animatable>(
        &self,
        key: &Hashed<FieldPath>,
        state: &GraphState,
    ) -> Result<T, TrackError> {
        let track = self.tracks.get(key).ok_or(TrackError::MissingTrack)?;
        track.blend(state)
    }

    pub(crate) fn sample_property(
        &self,
        key: &Hashed<FieldPath>,
        state: &GraphState,
        output: &mut dyn Reflect,
    ) -> Result<(), TrackError> {
        let key = key.into();
        let track = self.tracks.get(key).ok_or(TrackError::MissingTrack)?;
        track.blend_via_reflect(state, output)
    }
}

pub(super) struct GraphClips {
    bones: HashMap<EntityPath, BoneId>,
    // Indexed by BoneId
    tracks: Vec<Bone>,
}

impl GraphClips {
    pub(super) fn add_clip(
        &mut self,
        clip_id: ClipId,
        clip: &AnimationClip,
    ) -> Result<(), TrackError> {
        // Verify that the types for each of the tracks are identical before adding any of the curves in.
        for (path, curve) in clip.curves.iter() {
            if let Some(bone) = self.find_bone(path.entity()) {
                let key = Hashed::new(path.field().clone());
                if let Some(track) = bone.tracks.get(&key) {
                    if curve.value_type_id() != track.value_type_id() {
                        return Err(TrackError::IncorrectType);
                    }
                }
            }
        }

        for (path, curve) in clip.curves.iter() {
            let bone_id = if let Some(bone_id) = self.bones.get(path.entity()) {
                *bone_id
            } else {
                let bone_id = BoneId(self.tracks.len());
                self.bones.insert(path.entity().clone(), bone_id);
                self.tracks.push(Bone {
                    id: bone_id,
                    entity: None,
                    tracks: Default::default(),
                });
                bone_id
            };

            let bone_tracks = &mut self.tracks[bone_id.0];
            let key = Hashed::new(path.field().clone());
            if let Some(track) = bone_tracks.tracks.get_mut(&key) {
                track.add_generic_curve(clip_id, curve.as_ref()).unwrap();
            } else {
                bone_tracks.tracks.insert(key, curve.into_track(clip_id));
            }
        }

        Ok(())
    }

    pub(super) fn bones(&self) -> impl Iterator<Item = &Bone> {
        self.tracks.iter()
    }

    pub(super) fn find_bone(&self, path: &EntityPath) -> Option<&Bone> {
        self.bones
            .get(&path)
            .copied()
            .map(|bone_id| &self.tracks[bone_id.0])
    }

    pub(super) fn find_bone_mut(&mut self, path: &EntityPath) -> Option<&mut Bone> {
        self.bones
            .get(&path)
            .copied()
            .map(|bone_id| &mut self.tracks[bone_id.0])
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
