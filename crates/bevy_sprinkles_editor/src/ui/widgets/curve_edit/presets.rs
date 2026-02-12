use bevy_sprinkles::prelude::{CurveEasing, CurveMode, CurvePoint, CurveTexture};

// tension values for power-based easings: tension = (1 - 1/exp) / 0.999
const QUAD_TENSION: f64 = 0.5005005005;
const CUBIC_TENSION: f64 = 0.6673340007;
const QUART_TENSION: f64 = 0.7507507508;
const QUINT_TENSION: f64 = 0.8008008008;

pub struct CurvePreset {
    pub name: &'static str,
    start_value: f64,
    mode: CurveMode,
    easing: CurveEasing,
    tension: f64,
}

impl CurvePreset {
    const fn new(name: &'static str, mode: CurveMode, easing: CurveEasing, tension: f64) -> Self {
        Self {
            name,
            start_value: 0.0,
            mode,
            easing,
            tension,
        }
    }

    const fn constant(name: &'static str) -> Self {
        Self {
            name,
            start_value: 1.0,
            mode: CurveMode::DoubleCurve,
            easing: CurveEasing::Power,
            tension: 0.0,
        }
    }

    pub fn to_curve(&self, range: bevy_sprinkles::prelude::ParticleRange) -> CurveTexture {
        CurveTexture::new(vec![
            CurvePoint::new(0.0, self.start_value),
            CurvePoint::new(1.0, 1.0)
                .with_mode(self.mode)
                .with_easing(self.easing)
                .with_tension(self.tension),
        ])
        .with_name(self.name)
        .with_range(range)
    }
}

pub const CURVE_PRESETS: &[CurvePreset] = &[
    CurvePreset::constant("Constant"),
    CurvePreset::new("Linear", CurveMode::DoubleCurve, CurveEasing::Power, 0.0),
    CurvePreset::new(
        "Quad in",
        CurveMode::SingleCurve,
        CurveEasing::Power,
        QUAD_TENSION,
    ),
    CurvePreset::new(
        "Quad out",
        CurveMode::SingleCurve,
        CurveEasing::Power,
        -QUAD_TENSION,
    ),
    CurvePreset::new(
        "Quad in out",
        CurveMode::DoubleCurve,
        CurveEasing::Power,
        QUAD_TENSION,
    ),
    CurvePreset::new(
        "Cubic in",
        CurveMode::SingleCurve,
        CurveEasing::Power,
        CUBIC_TENSION,
    ),
    CurvePreset::new(
        "Cubic out",
        CurveMode::SingleCurve,
        CurveEasing::Power,
        -CUBIC_TENSION,
    ),
    CurvePreset::new(
        "Cubic in out",
        CurveMode::DoubleCurve,
        CurveEasing::Power,
        CUBIC_TENSION,
    ),
    CurvePreset::new(
        "Quart in",
        CurveMode::SingleCurve,
        CurveEasing::Power,
        QUART_TENSION,
    ),
    CurvePreset::new(
        "Quart out",
        CurveMode::SingleCurve,
        CurveEasing::Power,
        -QUART_TENSION,
    ),
    CurvePreset::new(
        "Quart in out",
        CurveMode::DoubleCurve,
        CurveEasing::Power,
        QUART_TENSION,
    ),
    CurvePreset::new(
        "Quint in",
        CurveMode::SingleCurve,
        CurveEasing::Power,
        QUINT_TENSION,
    ),
    CurvePreset::new(
        "Quint out",
        CurveMode::SingleCurve,
        CurveEasing::Power,
        -QUINT_TENSION,
    ),
    CurvePreset::new(
        "Quint in out",
        CurveMode::DoubleCurve,
        CurveEasing::Power,
        QUINT_TENSION,
    ),
    CurvePreset::new("Sine in", CurveMode::SingleCurve, CurveEasing::Sine, 1.0),
    CurvePreset::new("Sine out", CurveMode::SingleCurve, CurveEasing::Sine, -1.0),
    CurvePreset::new(
        "Sine in out",
        CurveMode::DoubleCurve,
        CurveEasing::Sine,
        1.0,
    ),
    CurvePreset::new("Expo in", CurveMode::SingleCurve, CurveEasing::Expo, 1.0),
    CurvePreset::new("Expo out", CurveMode::SingleCurve, CurveEasing::Expo, -1.0),
    CurvePreset::new(
        "Expo in out",
        CurveMode::DoubleCurve,
        CurveEasing::Expo,
        1.0,
    ),
    CurvePreset::new("Circ in", CurveMode::SingleCurve, CurveEasing::Circ, 1.0),
    CurvePreset::new("Circ out", CurveMode::SingleCurve, CurveEasing::Circ, -1.0),
    CurvePreset::new(
        "Circ in out",
        CurveMode::DoubleCurve,
        CurveEasing::Circ,
        1.0,
    ),
];
