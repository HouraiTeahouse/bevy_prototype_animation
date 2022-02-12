use crate::Sample;

use parallel_vec::ParallelVec;
use std::ops::{Add, Mul};

fn repeat(time: f32, length: f32) -> f32 {
    (time - f32::floor(time / length)).clamp(0.0, length)
}

fn ping_pong(time: f32, length: f32) -> f32 {
    let time = repeat(time, length * 2.0);
    length - (time - length).abs()
}

pub struct MixerNode<T: 'static> {
    inputs: ParallelVec<(f32, Box<dyn Sample<T>>)>,
}

impl<T> Sample<T> for MixerNode<T>
where
    T: Default + Add<Output = T> + Mul<f32, Output = T> + 'static,
{
    fn sample(&self, time: f32) -> T {
        let mut value = T::default();
        for (weight, input) in self.inputs.iter() {
            if *weight != 0.0 {
                value = value + input.sample(time) * (*weight);
            }
        }
        value
    }
}

impl<T: 'static> MixerNode<T> {
    pub fn new() -> Self {
        Self {
            inputs: ParallelVec::new(),
        }
    }

    pub fn add_input(&mut self, weight: f32, input: impl Sample<T> + 'static) {
        self.inputs.push((weight, Box::new(input)))
    }

    pub fn input_count(&self) -> usize {
        self.inputs.len()
    }

    pub fn clear(&mut self) {
        self.inputs.clear()
    }
}

pub struct ConstantNode<T: Clone>(pub T);

impl<T> Sample<T> for ConstantNode<T>
where
    T: Clone,
{
    fn sample(&self, _: f32) -> T {
        self.0.clone()
    }
}

pub struct RepeatNode<T: 'static> {
    length: f32,
    sub: Box<dyn Sample<T>>,
}

impl<T: 'static> Sample<T> for RepeatNode<T> {
    fn sample(&self, time: f32) -> T {
        self.sub.sample(repeat(time, self.length))
    }
}

impl<T: 'static> RepeatNode<T> {
    pub fn new(length: f32, sampler: impl Sample<T> + 'static) -> Self {
        assert!(length >= 0.0, "RepeatNode: Length must be non-negative.");
        Self {
            length,
            sub: Box::new(sampler),
        }
    }
}

pub struct PingPongNode<T> {
    length: f32,
    sub: Box<dyn Sample<T>>,
}

impl<T> Sample<T> for PingPongNode<T> {
    fn sample(&self, time: f32) -> T {
        self.sub.sample(ping_pong(time, self.length))
    }
}

impl<T> PingPongNode<T> {
    pub fn new(length: f32, sampler: impl Sample<T> + 'static) -> Self {
        assert!(length >= 0.0, "PingPongNode: Length must be non-negative.");
        Self {
            length,
            sub: Box::new(sampler),
        }
    }
}
