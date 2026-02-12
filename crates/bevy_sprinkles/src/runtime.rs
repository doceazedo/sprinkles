use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use bevy::render::render_resource::{Buffer, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;
use bytemuck::{Pod, Zeroable};

use crate::asset::{DrawPassMaterial, ParticleMesh, ParticleSystemAsset, ParticlesColliderShape3D};
use crate::material::ParticleMaterialExtension;

#[derive(Component)]
pub struct ParticleSystem2D {
    pub handle: Handle<ParticleSystemAsset>,
}

#[derive(Component)]
pub struct ParticleSystem3D {
    pub handle: Handle<ParticleSystemAsset>,
}

#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct ParticleData {
    pub position: [f32; 4],      // xyz + scale
    pub velocity: [f32; 4],      // xyz + lifetime_remaining
    pub color: [f32; 4],         // rgba
    pub custom: [f32; 4],        // age, phase, seed, flags
    pub alignment_dir: [f32; 4], // xyz direction for ALIGN_Y_TO_VELOCITY, w unused
}

impl ParticleData {
    pub const FLAG_ACTIVE: u32 = 1;

    pub fn is_active(&self) -> bool {
        let flags = self.custom[3].to_bits();
        (flags & Self::FLAG_ACTIVE) != 0
    }
}

#[derive(Component)]
pub struct ParticleSystemRuntime {
    pub paused: bool,
    pub force_loop: bool,
    pub global_seed: u32,
}

impl Default for ParticleSystemRuntime {
    fn default() -> Self {
        Self {
            paused: false,
            force_loop: true,
            global_seed: rand_seed(),
        }
    }
}

impl ParticleSystemRuntime {
    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn toggle(&mut self) {
        self.paused = !self.paused;
    }
}

#[derive(Clone, Copy)]
pub struct SimulationStep {
    pub prev_system_time: f32,
    pub system_time: f32,
    pub cycle: u32,
    pub delta_time: f32,
    pub clear_requested: bool,
}

#[derive(Component)]
pub struct EmitterRuntime {
    pub emitting: bool,
    pub system_time: f32,
    pub prev_system_time: f32,
    pub cycle: u32,
    pub accumulated_delta: f32,
    pub random_seed: u32,
    pub one_shot_completed: bool,
    pub clear_requested: bool,
    pub emitter_index: usize,
    pub simulation_steps: Vec<SimulationStep>,
}

impl EmitterRuntime {
    pub fn new(emitter_index: usize, fixed_seed: Option<u32>) -> Self {
        let random_seed = fixed_seed.unwrap_or_else(rand_seed);
        Self {
            emitting: true,
            system_time: 0.0,
            prev_system_time: 0.0,
            cycle: 0,
            accumulated_delta: 0.0,
            random_seed,
            one_shot_completed: false,
            clear_requested: false,
            emitter_index,
            simulation_steps: Vec::new(),
        }
    }

    pub fn system_phase(&self, time: &crate::asset::EmitterTime) -> f32 {
        if time.lifetime <= 0.0 {
            return 0.0;
        }
        let total_duration = time.total_duration();
        if total_duration <= 0.0 {
            return 0.0;
        }
        let time_in_cycle = self.system_time % total_duration;
        if time_in_cycle < time.delay {
            return 0.0;
        }
        (time_in_cycle - time.delay) / time.lifetime
    }

    pub fn prev_system_phase(&self, time: &crate::asset::EmitterTime) -> f32 {
        if time.lifetime <= 0.0 {
            return 0.0;
        }
        let total_duration = time.total_duration();
        if total_duration <= 0.0 {
            return 0.0;
        }
        let time_in_cycle = self.prev_system_time % total_duration;
        if time_in_cycle < time.delay {
            return 0.0;
        }
        (time_in_cycle - time.delay) / time.lifetime
    }

    pub fn is_past_delay(&self, time: &crate::asset::EmitterTime) -> bool {
        let total_duration = time.total_duration();
        if total_duration <= 0.0 {
            return true;
        }
        let time_in_cycle = self.system_time % total_duration;
        time_in_cycle >= time.delay
    }

    pub fn play(&mut self) {
        self.emitting = true;
        self.one_shot_completed = false;
    }

    pub fn stop(&mut self, fixed_seed: Option<u32>) {
        self.emitting = false;
        self.system_time = 0.0;
        self.prev_system_time = 0.0;
        self.cycle = 0;
        self.accumulated_delta = 0.0;
        self.random_seed = fixed_seed.unwrap_or_else(rand_seed);
        self.one_shot_completed = false;
        self.clear_requested = true;
        self.simulation_steps.clear();
    }

    pub fn restart(&mut self, fixed_seed: Option<u32>) {
        self.stop(fixed_seed);
        self.emitting = true;
    }

    pub fn seek(&mut self, time: f32) {
        self.system_time = time;
        self.prev_system_time = time;
    }
}

pub fn compute_phase(time: f32, emitter_time: &crate::asset::EmitterTime) -> f32 {
    if emitter_time.lifetime <= 0.0 {
        return 0.0;
    }
    let total_duration = emitter_time.total_duration();
    if total_duration <= 0.0 {
        return 0.0;
    }
    let time_in_cycle = time % total_duration;
    if time_in_cycle < emitter_time.delay {
        return 0.0;
    }
    (time_in_cycle - emitter_time.delay) / emitter_time.lifetime
}

pub fn is_past_delay(time: f32, emitter_time: &crate::asset::EmitterTime) -> bool {
    let total_duration = emitter_time.total_duration();
    if total_duration <= 0.0 {
        return true;
    }
    let time_in_cycle = time % total_duration;
    time_in_cycle >= emitter_time.delay
}

#[derive(Component)]
pub struct EmitterEntity {
    pub parent_system: Entity,
}

#[derive(Component)]
pub struct ColliderEntity {
    pub parent_system: Entity,
    pub collider_index: usize,
}

fn rand_seed() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (duration.as_nanos() & 0xFFFFFFFF) as u32
}

#[derive(Component)]
pub struct ParticleBufferHandle {
    pub particle_buffer: Handle<ShaderStorageBuffer>,
    pub indices_buffer: Handle<ShaderStorageBuffer>,
    pub sorted_particles_buffer: Handle<ShaderStorageBuffer>,
    pub max_particles: u32,
}

#[derive(Component)]
pub struct ParticleGpuBuffers {
    pub particle_buffer: Buffer,
    pub uniform_buffer: Buffer,
    pub max_particles: u32,
}

#[derive(Component)]
pub struct EmitterMeshEntity {
    pub emitter_entity: Entity,
}

#[derive(Component)]
pub struct CurrentMeshConfig(pub ParticleMesh);

#[derive(Component)]
pub struct CurrentMaterialConfig(pub DrawPassMaterial);

#[derive(Component)]
pub struct ParticleMeshHandle(pub Handle<Mesh>);

pub type ParticleMaterial = ExtendedMaterial<StandardMaterial, ParticleMaterialExtension>;

#[derive(Component)]
pub struct ParticleMaterialHandle(pub Handle<ParticleMaterial>);

#[derive(Component)]
pub struct SubEmitterBufferHandle {
    pub buffer: Handle<ShaderStorageBuffer>,
    pub target_emitter: Entity,
    pub max_particles: u32,
}

#[derive(Component, Debug, Clone)]
pub struct ParticlesCollider3D {
    pub enabled: bool,
    pub shape: ParticlesColliderShape3D,
    pub position: Vec3,
}

impl Default for ParticlesCollider3D {
    fn default() -> Self {
        Self {
            enabled: true,
            shape: ParticlesColliderShape3D::default(),
            position: Vec3::ZERO,
        }
    }
}
