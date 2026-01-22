use bevy::prelude::*;

use crate::asset::ParticleSystemAsset;

#[derive(Component)]
pub struct ParticleSystem2D {
    pub handle: Handle<ParticleSystemAsset>,
}
