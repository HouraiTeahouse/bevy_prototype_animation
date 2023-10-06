use crate::{
    clip::AnimationClip,
    clip::{ClipCurve, CurveWrapper},
    curve::Curve,
    graph::GraphState,
    path::{AccessPath, EntityPath},
    Animatable, BlendInput,
};
use bevy_ecs::prelude::{Entity, World};
use bevy_reflect::Reflect;
use bevy_utils::HashMap;
use std::{
    any::{Any, TypeId},
    collections::BTreeMap,
    sync::Arc,
};

pub(crate) struct BoneTrack<'a> {
    pub property: &'a AccessPath,
    pub track: &'a (dyn Track + 'static),
}

#[derive(Debug, Clone, Copy)]
pub struct BoneId(usize);

pub struct Bone {
    pub(super) id: BoneId,
    pub(super) path: EntityPath,
    pub(super) entity: Option<Entity>,
    // BTreeMap is used here as it's iteration is O(size) not O(capacity).
    // like HashMap. The lexographic ordering of FieldPath also ensures that the
    // fields on the same component applied close together during application.
    pub(super) tracks: BTreeMap<AccessPath, Box<dyn Track + 'static>>,
}

impl Bone {
    pub fn id(&self) -> BoneId {
        self.id
    }

    pub fn properties(&self) -> impl Iterator<Item = &AccessPath> {
        self.tracks.keys()
    }

    pub(crate) fn tracks(&self) -> impl Iterator<Item = BoneTrack<'_>> {
        self.tracks.iter().map(|(key, value)| BoneTrack {
            property: &key,
            track: value.as_ref(),
        })
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
}

pub(super) struct GraphClips {
    bones: HashMap<EntityPath, BoneId>,
    // Indexed by BoneId
    tracks: Vec<Bone>,
    pub(super) dirty: bool,
}

impl GraphClips {
    #[inline(always)]
    pub(super) fn is_dirty(&self) -> bool {
        self.dirty
    }

    #[inline(always)]
    pub(super) fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    pub(super) fn add_clip(
        &mut self,
        clip_id: ClipId,
        clip: &AnimationClip,
    ) -> Result<(), TrackError> {
        // Verify that the types for each of the tracks are identical before adding any of the curves in.
        for (path, curve) in clip.curves.iter() {
            let valid = self
                .find_bone(path.entity())
                .and_then(|bone| bone.tracks.get(path.access()))
                .map(|track| curve.value_type_id() == track.value_type_id())
                .unwrap_or(true);

            if !valid {
                return Err(TrackError::IncorrectType);
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
                    path: path.entity().clone(),
                    entity: None,
                    tracks: Default::default(),
                });
                self.dirty = true;
                bone_id
            };

            let bone_tracks = &mut self.tracks[bone_id.0];
            if let Some(track) = bone_tracks.tracks.get_mut(path.access()) {
                track.add_generic_curve(clip_id, curve.as_ref()).unwrap();
            } else {
                bone_tracks
                    .tracks
                    .insert(path.access().clone(), curve.into_track(clip_id));
            }
        }

        Ok(())
    }

    pub(super) fn get_bone(&self, id: BoneId) -> Option<&Bone> {
        self.tracks.get(id.0)
    }

    pub(super) fn bones(&self) -> impl Iterator<Item = &Bone> {
        self.tracks.iter()
    }

    pub(super) fn bones_mut(&mut self) -> impl Iterator<Item = &mut Bone> {
        self.tracks.iter_mut()
    }

    pub(super) fn find_bone(&self, path: &EntityPath) -> Option<&Bone> {
        self.bones
            .get(path)
            .copied()
            .map(|bone_id| &self.tracks[bone_id.0])
    }

    pub(super) fn find_bone_mut(&mut self, path: &EntityPath) -> Option<&mut Bone> {
        self.bones
            .get(path)
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

    /// Blends all of the values in the track and then postprocesses the
    /// result using the provided [`World`] reference.
    ///
    /// # Safety
    /// The provided [`World`] cannot have be mutated on a different thread.
    unsafe fn blend_via_reflect(
        &self,
        state: &GraphState,
        output: &mut dyn Reflect,
        world: &World,
    ) -> Result<(), TrackError>;
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

    unsafe fn blend_via_reflect(
        &self,
        state: &GraphState,
        output: &mut dyn Reflect,
        world: &World,
    ) -> Result<(), TrackError> {
        if output.as_any().type_id() == TypeId::of::<T>() {
            let mut value = self.sample_and_blend(state);
            if !matches!(value.reflect_partial_eq(output), Some(true)) {
                // SAFE: Only read-only access to the World's resources is
                // used here. No mutation nor reading of component/entity
                // data is done, as required by Animatable::post_process.
                value.post_process(world);
                output.apply(&value);
            }
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
