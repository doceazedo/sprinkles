use std::collections::HashMap;

use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use bevy::render::render_resource::{Buffer, ShaderType};
use bevy::render::storage::ShaderStorageBuffer;
use bytemuck::{Pod, Zeroable};

use crate::asset::{DrawPassMaterial, ParticleMesh, ParticleSystemAsset, ParticlesColliderShape3D};
use crate::material::ParticleMaterialExtension;

#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub(crate) struct TrailHistoryEntry {
    pub(crate) position: [f32; 4],
    pub(crate) velocity: [f32; 4],
}

/// Component that spawns a 2D particle system from a [`ParticleSystemAsset`].
///
/// # TODO
///
/// 2D particle systems are not yet implemented. This component exists as a
/// placeholder; spawning it will have no effect. Use [`ParticleSystem3D`] instead.
#[derive(Component)]
pub struct ParticleSystem2D {
    /// Handle to the particle system asset that defines this effect.
    pub handle: Handle<ParticleSystemAsset>,
}

/// Component that spawns a 3D particle system from a [`ParticleSystemAsset`].
#[derive(Component)]
pub struct ParticleSystem3D {
    /// Handle to the particle system asset that defines this effect.
    pub handle: Handle<ParticleSystemAsset>,
}

/// GPU-side per-particle data, packed into `[f32; 4]` vectors for shader alignment.
#[derive(Clone, Copy, Default, Pod, Zeroable, ShaderType)]
#[repr(C)]
pub struct ParticleData {
    /// XYZ position and W = scale.
    pub position: [f32; 4],
    /// XYZ velocity and W = remaining lifetime.
    pub velocity: [f32; 4],
    /// RGBA color.
    pub color: [f32; 4],
    /// X = age, Y = phase, Z = seed, W = flags (bit-packed).
    pub custom: [f32; 4],
    /// XYZ direction for velocity-aligned transforms, W = angle (radians).
    pub alignment_dir: [f32; 4],
    /// XYZ reference "up" direction for parallel-transported velocity alignment.
    pub ref_up: [f32; 4],
}

impl ParticleData {
    /// Bit flag indicating that a particle is alive and should be rendered.
    pub const FLAG_ACTIVE: u32 = 1;

    /// Returns `true` if this particle is currently active.
    pub fn is_active(&self) -> bool {
        let flags = self.custom[3].to_bits();
        (flags & Self::FLAG_ACTIVE) != 0
    }
}

/// Triggered when all active emitters have finished processing.
///
/// This event is only fired when **every** emitter in the system has
/// [`one_shot`](crate::EmitterTime::one_shot) set to `true`.
#[derive(EntityEvent)]
pub struct Finished(pub Entity);

/// Runtime state for a particle system entity, controlling playback.
#[derive(Component)]
pub struct ParticleSystemRuntime {
    /// Whether the simulation is paused. Defaults to `false`.
    pub paused: bool,
    /// Whether one-shot emitters should loop continuously. Defaults to `true`.
    pub force_loop: bool,
    /// Global random seed for all emitters in this system.
    pub global_seed: u32,
    pub(crate) finished: bool,
}

impl Default for ParticleSystemRuntime {
    fn default() -> Self {
        Self {
            paused: false,
            force_loop: true,
            global_seed: rand_seed(),
            finished: false,
        }
    }
}

impl ParticleSystemRuntime {
    /// Pauses the particle simulation.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resumes the particle simulation.
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Toggles between paused and playing.
    pub fn toggle(&mut self) {
        self.paused = !self.paused;
    }
}

/// A single simulation step to be processed by the compute shader.
#[derive(Clone, Copy)]
pub struct SimulationStep {
    /// System time at the start of this step.
    pub prev_system_time: f32,
    /// System time at the end of this step.
    pub system_time: f32,
    /// The current emission cycle index.
    pub cycle: u32,
    /// Duration of this simulation step in seconds.
    pub delta_time: f32,
    /// Whether to clear all particles before this step.
    pub clear_requested: bool,
    /// Snapshot of the trail history ring buffer write position for this step.
    pub trail_history_write_index: u32,
}

/// Runtime state for a single emitter within a particle system.
#[derive(Component)]
pub struct EmitterRuntime {
    /// Whether this emitter is actively spawning particles.
    pub(crate) emitting: bool,
    /// Current simulation time in seconds.
    pub system_time: f32,
    /// Simulation time from the previous frame.
    pub prev_system_time: f32,
    /// Current emission cycle index (increments each time the lifetime wraps).
    pub cycle: u32,
    /// Accumulated time delta for fixed-FPS stepping.
    pub accumulated_delta: f32,
    /// Random seed for this emitter's particle generation.
    pub random_seed: u32,
    /// Whether a one-shot emission cycle has completed.
    pub one_shot_completed: bool,
    /// Whether this emitter is fully idle (not emitting and all particles are dead).
    pub inactive: bool,
    /// Accumulated real time since the emitter stopped emitting.
    pub inactive_time: f32,
    /// Whether to clear all particles on the next frame.
    pub clear_requested: bool,
    /// Index of this emitter within the parent [`ParticleSystemAsset::emitters`].
    pub emitter_index: usize,
    /// Pending simulation steps to be dispatched to the GPU.
    pub simulation_steps: Vec<SimulationStep>,
    /// Current write position in the per-particle trail history ring buffer.
    pub trail_history_write_index: u32,
    /// Ring buffer size per particle for trail history.
    pub trail_history_frames: u32,
}

impl EmitterRuntime {
    /// Creates a new emitter runtime for the emitter at the given index.
    ///
    /// If `fixed_seed` is provided, it is used for deterministic behavior;
    /// otherwise a random seed is generated.
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
            inactive: false,
            inactive_time: 0.0,
            clear_requested: false,
            emitter_index,
            simulation_steps: Vec::new(),
            trail_history_write_index: 0,
            trail_history_frames: 0,
        }
    }

    /// Returns the current phase within the emission cycle, from `0.0` to `1.0`.
    pub fn system_phase(&self, time: &crate::asset::EmitterTime) -> f32 {
        compute_phase(self.system_time, time)
    }

    /// Returns the phase from the previous frame.
    pub fn prev_system_phase(&self, time: &crate::asset::EmitterTime) -> f32 {
        compute_phase(self.prev_system_time, time)
    }

    /// Returns `true` if the emitter has passed its initial delay within the current cycle.
    pub fn is_past_delay(&self, time: &crate::asset::EmitterTime) -> bool {
        is_past_delay(self.system_time, time)
    }

    /// Returns `true` if the emitter is actively spawning particles.
    pub fn is_emitting(&self) -> bool {
        self.emitting
    }

    pub(crate) fn set_emitting(&mut self, emitting: bool) {
        self.emitting = emitting;
        if emitting {
            self.inactive = false;
            self.inactive_time = 0.0;
        }
    }

    /// Starts or resumes emission, resetting the one-shot completed flag.
    pub fn play(&mut self) {
        self.set_emitting(true);
        self.one_shot_completed = false;
    }

    /// Stops emission and resets all timing state. Clears existing particles.
    pub fn stop(&mut self, fixed_seed: Option<u32>) {
        self.set_emitting(false);
        self.inactive = true;
        self.system_time = 0.0;
        self.prev_system_time = 0.0;
        self.cycle = 0;
        self.accumulated_delta = 0.0;
        self.random_seed = fixed_seed.unwrap_or_else(rand_seed);
        self.one_shot_completed = false;
        self.clear_requested = true;
        self.simulation_steps.clear();
        self.trail_history_write_index = 0;
    }

    pub(crate) fn advance_trail_history(&mut self) {
        if self.trail_history_frames > 0 {
            self.trail_history_write_index =
                (self.trail_history_write_index + 1) % self.trail_history_frames;
        }
    }

    /// Stops and immediately restarts emission from the beginning.
    pub fn restart(&mut self, fixed_seed: Option<u32>) {
        self.stop(fixed_seed);
        self.set_emitting(true);
    }

    /// Jumps the emitter's simulation time to the given value.
    pub fn seek(&mut self, time: f32) {
        self.system_time = time;
        self.prev_system_time = time;
    }
}

/// Computes the emission phase (0.0–1.0) for the given time and emitter timing config.
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

/// Returns `true` if the given time is past the emitter's initial delay within the current cycle.
pub fn is_past_delay(time: f32, emitter_time: &crate::asset::EmitterTime) -> bool {
    let total_duration = emitter_time.total_duration();
    if total_duration <= 0.0 {
        return true;
    }
    let time_in_cycle = time % total_duration;
    time_in_cycle >= emitter_time.delay
}

/// Marker component linking an emitter entity back to its parent particle system.
#[derive(Component)]
pub struct EmitterEntity {
    /// The entity that holds the [`ParticleSystem3D`] or [`ParticleSystem2D`] component.
    pub parent_system: Entity,
}

/// Marker component linking a collider entity back to its parent particle system.
#[derive(Component)]
pub struct ColliderEntity {
    /// The entity that holds the [`ParticleSystem3D`] or [`ParticleSystem2D`] component.
    pub parent_system: Entity,
    /// Index of this collider within the parent [`ParticleSystemAsset::colliders`].
    pub collider_index: usize,
}

fn rand_seed() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (duration.as_nanos() & 0xFFFFFFFF) as u32
}

/// Handles to the GPU storage buffers for an emitter's particle data.
#[derive(Component)]
pub struct ParticleBufferHandle {
    /// The main particle data buffer.
    pub particle_buffer: Handle<ShaderStorageBuffer>,
    /// Buffer holding particle sort indices.
    pub indices_buffer: Handle<ShaderStorageBuffer>,
    /// Buffer holding sorted/reordered particle data for rendering.
    pub sorted_particles_buffer: Handle<ShaderStorageBuffer>,
    /// Buffer holding per-emitter uniforms (transform, flags) for the material shader.
    pub emitter_uniforms_buffer: Handle<ShaderStorageBuffer>,
    /// Maximum number of particles this buffer can hold (amount * trail_size).
    pub max_particles: u32,
    /// The particle count used when allocating buffers.
    pub amount: u32,
    /// The trail size used when allocating buffers.
    pub trail_size: u32,
    /// Per-particle trail position history ring buffer.
    pub trail_history_buffer: Option<Handle<ShaderStorageBuffer>>,
    /// Ring buffer size per particle for trail history.
    pub trail_history_frames: u32,
}

/// Raw GPU buffer references for an emitter, used during compute dispatch.
#[derive(Component)]
pub struct ParticleGpuBuffers {
    /// The main particle data buffer on the GPU.
    pub particle_buffer: Buffer,
    /// The simulation uniform buffer on the GPU.
    pub uniform_buffer: Buffer,
    /// Maximum number of particles this buffer can hold.
    pub max_particles: u32,
}

/// Tracks the currently applied mesh configuration for change detection.
#[derive(Component)]
pub struct CurrentMeshConfig(pub ParticleMesh);

/// Tracks the currently applied material configuration for change detection.
#[derive(Component)]
pub struct CurrentMaterialConfig(pub DrawPassMaterial);

/// Handle to the mesh used for rendering particles.
#[derive(Component)]
pub struct ParticleMeshHandle(pub Handle<Mesh>);

/// Type alias for the extended particle material used in rendering.
pub type ParticleMaterial = ExtendedMaterial<StandardMaterial, ParticleMaterialExtension>;

/// Handle to the particle material asset.
#[derive(Component)]
pub struct ParticleMaterialHandle(pub Handle<ParticleMaterial>);

/// Buffer handle for sub-emitter data exchange between parent and child emitters.
#[derive(Component)]
pub struct SubEmitterBufferHandle {
    /// Handle to the sub-emitter event buffer.
    pub buffer: Handle<ShaderStorageBuffer>,
    /// The target emitter entity that receives sub-emitter events.
    pub target_emitter: Entity,
    /// Maximum number of particles the target emitter can hold.
    pub max_particles: u32,
}

/// A 3D collider that particles can interact with at runtime.
///
/// Add this component to an entity (alongside a [`Transform`]) to create a collision
/// surface for particles. The collision behavior depends on each emitter's
/// [`EmitterCollision`](crate::EmitterCollision) settings.
#[derive(Component, Debug, Clone)]
pub struct ParticlesCollider3D {
    /// Whether this collider is active.
    pub enabled: bool,
    /// The collision shape.
    pub shape: ParticlesColliderShape3D,
}

impl Default for ParticlesCollider3D {
    fn default() -> Self {
        Self {
            enabled: true,
            shape: ParticlesColliderShape3D::default(),
        }
    }
}

pub(crate) fn check_particle_system_finished(
    mut commands: Commands,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut system_query: Query<(Entity, &ParticleSystem3D, &mut ParticleSystemRuntime)>,
    emitter_query: Query<(&EmitterEntity, &EmitterRuntime)>,
) {
    let mut system_states: HashMap<Entity, (bool, bool)> = HashMap::new();
    for (emitter, runtime) in emitter_query.iter() {
        let (any_emitting, all_finished) = system_states
            .entry(emitter.parent_system)
            .or_insert((false, true));
        if runtime.emitting {
            *any_emitting = true;
        }
        if !runtime.one_shot_completed {
            *all_finished = false;
        }
    }

    for (system_entity, particle_system, mut system_runtime) in system_query.iter_mut() {
        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        if !asset.emitters.iter().all(|e| e.time.one_shot) {
            continue;
        }

        let Some(&(any_emitting, all_finished)) = system_states.get(&system_entity) else {
            continue;
        };

        if system_runtime.finished {
            if any_emitting {
                system_runtime.finished = false;
            }
            continue;
        }

        if all_finished {
            commands.entity(system_entity).trigger(Finished);
            system_runtime.finished = true;

            if asset.despawn_on_finish {
                commands.entity(system_entity).despawn();
            }
        }
    }
}
