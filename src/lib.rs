pub mod curve;
mod math;

use curve::*;

use std::borrow::Cow;
use std::collections::HashMap;

pub struct AnimationClip {
    curves: HashMap<Cow<'static, str>, AnimationCurve>,
}

pub trait Sample<T> {
    fn sample(&self, time: f32) -> T;
}
