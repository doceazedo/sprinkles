use std::time::{SystemTime, UNIX_EPOCH};

use bevy::ecs::schedule::{ScheduleBuildSettings, ScheduleLabel};
use bevy::prelude::*;

const SEED_ENV_VAR: &str = "SPRINKLES_SHUFFLE_SEED";

pub fn plugin(app: &mut App) {
    let seed = std::env::var(SEED_ENV_VAR)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos() as u64)
                .unwrap_or_default()
        });

    info!("Shuffling Update and PostUpdate schedules with {SEED_ENV_VAR}={seed}");

    for label in [Update.intern(), PostUpdate.intern()] {
        app.edit_schedule(label, |schedule| {
            schedule.set_build_settings(ScheduleBuildSettings {
                shuffle_seed: Some(seed),
                ..schedule.get_build_settings()
            });
        });
    }
}
