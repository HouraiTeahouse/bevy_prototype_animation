pub mod curves;
pub mod graph;
pub mod math;

use curves::Curve;
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

pub trait Sample<T> {
    fn sample(&self, time: f32) -> T;
}

/// An immutable container of curves.
pub struct AnimationClip {
    // Actually a map of str -> Arc<dyn Curve<T>>
    curves: HashMap<Cow<'static, str>, Box<dyn Any>>,
}

impl AnimationClip {
    pub fn builder() -> AnimationClipBuilder {
        AnimationClipBuilder::new()
    }

    pub fn get_curve<T: 'static>(
        &self,
        key: impl Into<Cow<'static, str>>,
    ) -> Result<Arc<dyn Curve<T>>, GetCurveError> {
        self.curves
            .get(&key.into())
            .ok_or(GetCurveError::MissingKey)
            .and_then(|curve| {
                curve
                    .downcast_ref::<Arc<dyn Curve<T>>>()
                    .map(|curve| curve.clone())
                    .ok_or(GetCurveError::WrongType)
            })
    }
}

pub struct AnimationClipBuilder {
    // Actually a map of str -> Arc<dyn Curve<T>>
    curves: HashMap<Cow<'static, str>, Box<dyn Any>>,
}

impl AnimationClipBuilder {
    pub fn new() -> AnimationClipBuilder {
        Self {
            curves: HashMap::new(),
        }
    }

    pub fn add_curve<T: 'static>(
        self,
        key: impl Into<Cow<'static, str>>,
        curve: impl Curve<T> + 'static,
    ) -> Self {
        self.add_dynamic_curve(key, Arc::new(curve))
    }

    pub fn add_dynamic_curve<T: 'static>(
        mut self,
        key: impl Into<Cow<'static, str>>,
        curve: Arc<dyn Curve<T>>,
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
