use bevy::prelude::*;

use crate::{
    asset::ParticleSystemAsset,
    core::ParticleSystem3D,
    runtime::ParticleSystemRuntime,
};

pub fn update_particle_time(
    time: Res<Time>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut query: Query<(&ParticleSystem3D, &mut ParticleSystemRuntime)>,
) {
    for (particle_system, mut runtime) in query.iter_mut() {
        let Some(asset) = assets.get(&particle_system.handle) else {
            continue;
        };

        let Some(emitter) = asset.emitters.first() else {
            continue;
        };

        if !runtime.emitting {
            continue;
        }

        let lifetime = emitter.time.lifetime;
        let fixed_fps = emitter.time.fixed_fps;

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

                // check for cycle wrap
                if runtime.system_time >= lifetime && lifetime > 0.0 {
                    runtime.system_time = runtime.system_time % lifetime;
                    runtime.cycle += 1;
                }
            }
        } else {
            // variable timestep mode
            runtime.system_time += time.delta_secs();

            // check for cycle wrap
            if runtime.system_time >= lifetime && lifetime > 0.0 {
                runtime.system_time = runtime.system_time % lifetime;
                runtime.cycle += 1;
            }
        }

        // handle one-shot mode
        if emitter.time.one_shot && runtime.cycle > 0 && !runtime.one_shot_completed {
            runtime.emitting = false;
            runtime.one_shot_completed = true;
        }
    }
}
