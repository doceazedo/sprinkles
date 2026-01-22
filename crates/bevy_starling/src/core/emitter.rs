use std::ops::Range;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitterConfig {
    pub emission_rate: f32,
    pub lifetime: Range<f32>,
    pub enabled: bool,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            emission_rate: 10.0,
            lifetime: 1.0..2.0,
            enabled: true,
        }
    }
}
