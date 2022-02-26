use crate::{
    curve::Curve,
    graph::{ClipId, CurveTrack, Track},
    path::PropertyPath,
    Animatable,
};
use bevy_reflect::TypeUuid;
use bevy_utils::{Hashed, PreHashMap};
use std::{
    any::{Any, TypeId},
    sync::Arc,
};

#[derive(Clone)]
pub(crate) struct CurveWrapper<T>(pub Arc<dyn Curve<T>>);

pub(crate) trait ClipCurve: Send + Sync + 'static {
    fn value_type_id(&self) -> TypeId;
    fn as_any(&self) -> &dyn Any;
    fn into_track(&self, clip_id: ClipId) -> Box<dyn Track>;
}

impl<T: Animatable> ClipCurve for CurveWrapper<T> {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    fn as_any(&self) -> &dyn Any {
        self as &_
    }
    fn into_track(&self, clip_id: ClipId) -> Box<dyn Track> {
        Box::new(CurveTrack::new(self.0.clone(), clip_id))
    }
}

/// An immutable container of curves.
#[derive(TypeUuid)]
#[uuid = "28258d17-82c2-4a6f-8930-322baa150396"]
pub struct AnimationClip {
    // TODO: See if we can remove this extra layer of indirection
    pub(crate) curves: PreHashMap<PropertyPath, Box<dyn ClipCurve>>,
}

impl AnimationClip {
    pub fn builder() -> AnimationClipBuilder {
        AnimationClipBuilder::new()
    }

    pub fn properties(&self) -> impl Iterator<Item = &Hashed<PropertyPath>> {
        self.curves.keys()
    }

    pub fn get_curve<T: Animatable + 'static>(
        &self,
        key: &Hashed<PropertyPath>,
    ) -> Result<Arc<dyn Curve<T>>, GetCurveError> {
        self.curves
            .get(key)
            .ok_or(GetCurveError::MissingKey)
            .and_then(|curve| {
                curve
                    .as_any()
                    .downcast_ref::<CurveWrapper<T>>()
                    .map(|wrapper| wrapper.0.clone())
                    .ok_or(GetCurveError::WrongType)
            })
    }
}

pub struct AnimationClipBuilder {
    curves: PreHashMap<PropertyPath, Box<dyn ClipCurve>>,
}

impl AnimationClipBuilder {
    pub fn new() -> AnimationClipBuilder {
        Self {
            curves: PreHashMap::default(),
        }
    }

    pub fn add_curve<T: Animatable + 'static>(
        self,
        key: impl Into<PropertyPath>,
        curve: impl Curve<T> + Send + Sync + 'static,
    ) -> Self {
        self.add_dynamic_curve(key, Arc::new(curve))
    }

    pub fn add_dynamic_curve<T: Animatable + 'static>(
        mut self,
        key: impl Into<PropertyPath>,
        curve: Arc<dyn Curve<T>>,
    ) -> Self {
        self.curves
            .insert(Hashed::new(key.into()), Box::new(CurveWrapper(curve)));
        self
    }

    pub fn build(self) -> AnimationClip {
        AnimationClip {
            curves: self.curves,
        }
    }
}

pub enum GetCurveError {
    MissingKey,
    WrongType,
}
