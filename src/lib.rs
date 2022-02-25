#[cfg(test)]
#[macro_use]
extern crate static_assertions;

mod animatable;
pub mod clip;
pub mod curve;
pub mod graph;
mod util;

pub use animatable::*;

pub mod prelude {
    pub use crate::{clip::AnimationClip, curve::Curve, graph::AnimationGraph};
}
