use bevy::prelude::*;
use bevy::render::render_resource::Buffer;
use bevy::render::storage::ShaderStorageBuffer;

use crate::asset::ParticleMesh;

#[derive(Component)]
pub struct ParticleSystemRuntime {
    pub emitting: bool,
    pub system_time: f32,
    pub prev_system_time: f32,
    pub cycle: u32,
    pub accumulated_delta: f32,
    pub random_seed: u32,
    /// set to true when a one-shot emitter completes its emission cycle
    pub one_shot_completed: bool,
    /// set to true when the simulation is paused (freezes physics)
    pub paused: bool,
    /// set to true to clear all particles on the next frame
    pub clear_requested: bool,
}

impl Default for ParticleSystemRuntime {
    fn default() -> Self {
        Self {
            emitting: true,
            system_time: 0.0,
            prev_system_time: 0.0,
            cycle: 0,
            accumulated_delta: 0.0,
            random_seed: rand_seed(),
            one_shot_completed: false,
            paused: false,
            clear_requested: false,
        }
    }
}

impl ParticleSystemRuntime {
    pub fn system_phase(&self, lifetime: f32) -> f32 {
        if lifetime <= 0.0 {
            return 0.0;
        }
        (self.system_time % lifetime) / lifetime
    }

    pub fn prev_system_phase(&self, lifetime: f32) -> f32 {
        if lifetime <= 0.0 {
            return 0.0;
        }
        (self.prev_system_time % lifetime) / lifetime
    }

    /// Start or resume playback
    pub fn play(&mut self) {
        self.emitting = true;
        self.paused = false;
        self.one_shot_completed = false;
    }

    /// Pause playback, freezing all particles in place
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Stop playback, reset time, and clear all particles
    pub fn stop(&mut self) {
        self.emitting = false;
        self.paused = false;
        self.system_time = 0.0;
        self.prev_system_time = 0.0;
        self.cycle = 0;
        self.accumulated_delta = 0.0;
        self.random_seed = rand_seed();
        self.one_shot_completed = false;
        self.clear_requested = true;
    }

    /// Restart playback from the beginning
    pub fn restart(&mut self) {
        self.stop();
        self.emitting = true;
    }
}

fn rand_seed() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (duration.as_nanos() & 0xFFFFFFFF) as u32
}

/// stores handles to particle storage buffer for GPU rendering
#[derive(Component)]
pub struct ParticleBufferHandle {
    pub particle_buffer: Handle<ShaderStorageBuffer>,
    pub indices_buffer: Handle<ShaderStorageBuffer>,
    pub max_particles: u32,
}

/// stores raw GPU buffers for compute shader access in render world
#[derive(Component)]
pub struct ParticleGpuBuffers {
    pub particle_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub max_particles: u32,
}

/// marker component for individual particle entities
#[derive(Component)]
pub struct ParticleEntity;

/// stores the parent particle system entity for cleanup purposes
#[derive(Component)]
pub struct ParticleSystemRef(pub Entity);

/// stores the current mesh configuration for change detection
#[derive(Component)]
pub struct CurrentMeshConfig(pub ParticleMesh);

/// stores the mesh handle for particle entities
#[derive(Component)]
pub struct ParticleMeshHandle(pub Handle<Mesh>);
