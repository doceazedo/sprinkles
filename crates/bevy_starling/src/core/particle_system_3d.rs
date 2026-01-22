use bevy::prelude::*;

use crate::asset::ParticleSystemAsset;

#[derive(Component)]
pub struct ParticleSystem3D {
    pub handle: Handle<ParticleSystemAsset>,
}
