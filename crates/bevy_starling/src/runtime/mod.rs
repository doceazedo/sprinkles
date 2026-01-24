use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use bevy::render::render_resource::Buffer;
use bevy::render::storage::ShaderStorageBuffer;

use crate::asset::ParticleMesh;
use crate::render::material::ParticleMaterialExtension;

/// system-wide runtime state for a particle system
#[derive(Component)]
pub struct ParticleSystemRuntime {
    /// set to true when the simulation is paused (freezes physics)
    pub paused: bool,
    pub global_seed: u32,
}

impl Default for ParticleSystemRuntime {
    fn default() -> Self {
        Self {
            paused: false,
            global_seed: rand_seed(),
        }
    }
}

impl ParticleSystemRuntime {
    /// Pause playback, freezing all particles in place
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume playback
    pub fn resume(&mut self) {
        self.paused = false;
    }
}

/// per-emitter runtime state
#[derive(Component)]
pub struct EmitterRuntime {
    pub emitting: bool,
    pub system_time: f32,
    pub prev_system_time: f32,
    pub cycle: u32,
    pub accumulated_delta: f32,
    pub random_seed: u32,
    /// set to true when a one-shot emitter completes its emission cycle
    pub one_shot_completed: bool,
    /// set to true to clear all particles on the next frame
    pub clear_requested: bool,
    /// index into the asset's emitters array
    pub emitter_index: usize,
}

impl EmitterRuntime {
    pub fn new(emitter_index: usize) -> Self {
        Self {
            emitting: true,
            system_time: 0.0,
            prev_system_time: 0.0,
            cycle: 0,
            accumulated_delta: 0.0,
            random_seed: rand_seed(),
            one_shot_completed: false,
            clear_requested: false,
            emitter_index,
        }
    }

    pub fn system_phase(&self, lifetime: f32, delay: f32) -> f32 {
        if lifetime <= 0.0 {
            return 0.0;
        }
        let total_duration = delay + lifetime;
        if total_duration <= 0.0 {
            return 0.0;
        }
        // during delay period, phase is 0
        let time_in_cycle = self.system_time % total_duration;
        if time_in_cycle < delay {
            return 0.0;
        }
        (time_in_cycle - delay) / lifetime
    }

    pub fn prev_system_phase(&self, lifetime: f32, delay: f32) -> f32 {
        if lifetime <= 0.0 {
            return 0.0;
        }
        let total_duration = delay + lifetime;
        if total_duration <= 0.0 {
            return 0.0;
        }
        // during delay period, phase is 0
        let time_in_cycle = self.prev_system_time % total_duration;
        if time_in_cycle < delay {
            return 0.0;
        }
        (time_in_cycle - delay) / lifetime
    }

    /// returns true if the emitter is currently past its delay period and should spawn particles
    pub fn is_past_delay(&self, lifetime: f32, delay: f32) -> bool {
        let total_duration = delay + lifetime;
        if total_duration <= 0.0 {
            return true;
        }
        let time_in_cycle = self.system_time % total_duration;
        time_in_cycle >= delay
    }

    /// Start or resume playback
    pub fn play(&mut self) {
        self.emitting = true;
        self.one_shot_completed = false;
    }

    /// Stop playback, reset time, and clear all particles
    pub fn stop(&mut self) {
        self.emitting = false;
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

/// marker component for emitter child entities
#[derive(Component)]
pub struct EmitterEntity {
    pub parent_system: Entity,
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
    /// sorted particle data for rendering (written in draw order)
    pub sorted_particles_buffer: Handle<ShaderStorageBuffer>,
    pub max_particles: u32,
}

/// stores raw GPU buffers for compute shader access in render world
#[derive(Component)]
pub struct ParticleGpuBuffers {
    pub particle_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub max_particles: u32,
}

/// links an emitter's rendering mesh entity to its emitter
#[derive(Component)]
pub struct EmitterMeshEntity {
    pub emitter_entity: Entity,
}

/// stores the current mesh configuration for change detection
#[derive(Component)]
pub struct CurrentMeshConfig(pub ParticleMesh);

/// stores the mesh handle for particle entities
#[derive(Component)]
pub struct ParticleMeshHandle(pub Handle<Mesh>);

pub type ParticleMaterial = ExtendedMaterial<StandardMaterial, ParticleMaterialExtension>;

/// stores the shared material handle for all particle entities in a system
#[derive(Component)]
pub struct ParticleMaterialHandle(pub Handle<ParticleMaterial>);
