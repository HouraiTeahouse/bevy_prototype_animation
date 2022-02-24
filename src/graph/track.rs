use crate::{Animatable, BlendWeight};

pub(crate) struct GraphClips {
    tracks: HashMap<Cow<'static, str>, TrackUntyped>
}

pub struct TrackError {
    IncorrectType,
}

/// A non-generic interface for all [`Track<T>`] that can be used to hide
/// the internal type-specific implementation.
trait Track: Any {
    fn add_curve(&mut self, clip_id: ClipId, curve: Arc<dyn Curve<T>>) -> Result<(), TrackError>;
    fn blend_and_apply(&self, state: &GraphState, output: &mut dyn Reflect) -> Result<(), TrackError>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ClipId(pub u16);

pub(crate) struct Track<T> {
    curves: Vec<Option<Arc<dyn Curve<T>>>>,
}

impl Track<T> {
    pub fn blend(&self, state: &GraphState) -> T {
        let inputs = self
            .state
            .iter()
            .zip(self.curves.iter())
            .filter(|(clip, curve)| clip.weight != 0.0 && curve.is_some())
            .map(|(clip, curve)| BlendInput {
                weight: clip.weight,
                value: curve.unwrap().sample(clip.time),
            });

        T::blend(inputs)
    }
}

impl<T> Track for Track<T> {
    pub fn add_curve<C>(
        &mut self,
        clip_id: ClipId,
        curve: Arc<dyn Curve<C>>
    ) -> Result<(), TrackError> {
        if TypeId::of::<T>() != TypeId::of::<C>() {
            return Err(TrackError::IncorrectType);
        }
        // SAFE: If T == C here, the memory layout and
        // vtable for both is the same.
        let curve = unsafe {
            std::mem::transmute::<Arc<dyn Curve<T>>>(curve)
        };
        let idx = clip_id.0 as usize;
        self.curves.fill_with(idx, || None);
        self.curves[idx] = Some(curve);
        Ok(())
    }

    fn blend_and_apply(
        &self,
        state: &GraphState,
        output: &mut dyn Reflect
    ) -> Result<(), TrackError> {
        let output = Self:blend(state);
        output.apply(&output)
              .map_err(|_| TrackError::IncorrectType)?;
        Ok(())
    }
}

/// A type-erased track of curves. For the concrete generic version
/// see [`Track<T>`].
pub(crate) struct TrackUntyped {
    track: Box<dyn Track>
}

impl TrackUntyped {
    /// Creates a type-erased track from a generic one.
    pub fn create<T>(track: Track<T>) -> Self {
        Self {
            track: Box::new(track)
        }
    }

    /// Attempts to downcast the track to a specific track type.
    pub fn downcast_ref<T>(&self) -> Option<&Track<T>> {
        self.track.downcast_ref::<Track<T>>().ok()
    }

    /// Adds a curve from a given [`ClipId`].
    ///
    /// Returns [`TrackError::IncorrectType`] if the generic type
    /// parameter `C` is not compatible with the underlying track.
    pub fn add_curve<C>(
        &mut self,
        clip_id: ClipId,
        curve: Arc<dyn Curve<C>>
    ) -> Result<(), TrackError> {
        self.track.add_curve(clip_id, curve)
    }

    /// Blends all of the track's inputs and writes the blended
    /// value out to a dynamic object.
    ///
    /// Returns [`TrackError::IncorrectType`] if the reflected value
    /// is not a type the blended output can be written to.
    pub fn blend(
        &self,
        state: &GraphState,
        output: &mut dyn Reflect
    ) -> Result<(), TrackError> {
        self.track.blend_and_apply(state, output)
    }
}
