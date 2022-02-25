pub enum CompressedFloat32Curve {
    Static {
        duration: f32,
        value: f32,
    },
    Quantized {
        frame_rate: f32,
        frames: Vec<u16>,
        min_value: f32,
        increment: f32,
    },
}

impl CompressedFloat32Curve {
    pub fn sample(&self, time: f32) -> f32 {
        match self {
            Self::Static { value, .. } => value,
            Self::Quantized { value, .. } => value,
        }
    }
}

pub struct CompressedFloat32x2Curve {
    x: CompressedFloat32Curve,
    y: CompressedFloat32Curve,
}

pub struct CompressedFloat32x3Curve {
    x: CompressedFloat32Curve,
    y: CompressedFloat32Curve,
    z: CompressedFloat32Curve,
}

pub struct CompressedFloat32x4Curve {
    x: CompressedFloat32Curve,
    y: CompressedFloat32Curve,
    z: CompressedFloat32Curve,
    w: CompressedFloat32Curve,
}
