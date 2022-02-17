pub mod curves;
pub mod graph;
pub mod math;

use curves::*;
use bevy_reflect::{Reflect, TypeUuid};
use bevy_utils::HashMap;
use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

/// An immutable container of curves.
#[derive(TypeUuid)]
#[uuid = "28258d17-82c2-4a6f-8930-322baa150396"]
pub struct AnimationClip {
    curves: HashMap<Cow<'static, str>, CurveUntyped>,
}

impl AnimationClip {
    pub fn builder() -> AnimationClipBuilder {
        AnimationClipBuilder::new()
    }

    pub fn get_curve<T: 'static>(
        &self,
        key: impl Into<Cow<'static, str>>,
    ) -> Result<Arc<dyn Curve<Output=T>>, GetCurveError> {
        self.curves
            .get(&key.into())
            .ok_or(GetCurveError::MissingKey)
            .and_then(|curve| {
                curve
                    .downcast_ref::<Arc<dyn Curve<Output=T>>>()
                    .map(|curve| curve.clone())
                    .ok_or(GetCurveError::WrongType)
            })
    }
}

pub struct AnimationClipBuilder {
    curves: HashMap<Cow<'static, str>, Arc<dyn Curve + Send + Sync + 'static>>,
}

impl AnimationClipBuilder {
    pub fn new() -> AnimationClipBuilder {
        Self {
            curves: HashMap::default(),
        }
    }

    pub fn add_curve<T: 'static>(
        self,
        key: impl Into<Cow<'static, str>>,
        curve: impl Curve<Output = T> + Send + Sync + 'static,
    ) -> Self {
        self.add_dynamic_curve(key, Arc::new(curve))
    }

    pub fn add_dynamic_curve<T: 'static>(
        mut self,
        key: impl Into<Cow<'static, str>>,
        curve: Arc<dyn Curve<Output = T> + Send + Sync + 'static>,
    ) -> Self {
        self.curves.insert(key.into(), Box::new(curve));
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
