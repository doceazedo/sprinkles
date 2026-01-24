use bevy::prelude::*;

use crate::{
    asset::ParticleSystemAsset,
    core::ParticleSystem3D,
    runtime::{EmitterEntity, EmitterRuntime, ParticleSystemRuntime},
};

pub fn clear_particle_clear_requests(mut query: Query<&mut EmitterRuntime>) {
    for mut runtime in query.iter_mut() {
        if runtime.clear_requested {
            runtime.clear_requested = false;
        }
    }
}

pub fn update_particle_time(
    time: Res<Time>,
    assets: Res<Assets<ParticleSystemAsset>>,
    system_query: Query<(&ParticleSystem3D, &ParticleSystemRuntime)>,
    mut emitter_query: Query<(&EmitterEntity, &mut EmitterRuntime)>,
) {
    for (emitter, mut runtime) in emitter_query.iter_mut() {
        let Ok((particle_system, system_runtime)) = system_query.get(emitter.parent_system) else {
            continue;
        };

        if system_runtime.paused {
            continue;
        }

        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        let Some(emitter_data) = asset.emitters.get(runtime.emitter_index) else {
            continue;
        };

        let lifetime = emitter_data.time.lifetime;
        let delay = emitter_data.time.delay;
        let fixed_fps = emitter_data.time.fixed_fps;
        let total_duration = delay + lifetime;

        // store previous time for phase calculation
        runtime.prev_system_time = runtime.system_time;

        if fixed_fps > 0 {
            // fixed timestep mode
            let fixed_delta = 1.0 / fixed_fps as f32;
            runtime.accumulated_delta += time.delta_secs();

            // advance time in fixed increments
            while runtime.accumulated_delta >= fixed_delta {
                runtime.accumulated_delta -= fixed_delta;
                runtime.system_time += fixed_delta;

                // check for cycle wrap (accounts for delay + lifetime)
                if runtime.system_time >= total_duration && total_duration > 0.0 {
                    runtime.system_time = runtime.system_time % total_duration;
                    runtime.cycle += 1;
                }
            }
        } else {
            // variable timestep mode
            runtime.system_time += time.delta_secs();

            // check for cycle wrap (accounts for delay + lifetime)
            if runtime.system_time >= total_duration && total_duration > 0.0 {
                runtime.system_time = runtime.system_time % total_duration;
                runtime.cycle += 1;
            }
        }

        // handle one-shot mode
        if emitter_data.time.one_shot && runtime.cycle > 0 && !runtime.one_shot_completed {
            runtime.emitting = false;
            runtime.one_shot_completed = true;
        }
    }
}
