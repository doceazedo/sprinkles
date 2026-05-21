use std::time::Duration;

use bevy::{light::light_consts::lux, log::LogPlugin, prelude::*, window::PresentMode};
use bevy_sprinkles::prelude::*;

const WARMUP_SECS: f32 = 3.0;
const MEASURE_FRAMES: u32 = 5000;
const SPACING: f32 = 3.0;
const GRID_SIZES: [i32; 3] = [1, 3, 5];

#[derive(Resource)]
struct BenchState {
    config_index: usize,
    frame: u32,
    measuring: bool,
    warmup_timer: Timer,
    total: Duration,
    results: Vec<(u32, Duration)>,
}

#[derive(Resource)]
struct RestartTimer(Timer);

#[derive(Component)]
struct BenchParticleSystem;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::AutoNoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .disable::<LogPlugin>(),
        )
        .add_plugins(SprinklesPlugin)
        .insert_resource(BenchState {
            config_index: 0,
            frame: 0,
            measuring: false,
            warmup_timer: Timer::from_seconds(WARMUP_SECS, TimerMode::Once),
            total: Duration::ZERO,
            results: Vec::new(),
        })
        .insert_resource(RestartTimer(Timer::from_seconds(3.0, TimerMode::Repeating)))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (restart_systems, bench_tick))
        .run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        AmbientLight::default(),
        DirectionalLight {
            illuminance: lux::OVERCAST_DAY,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));

    let grid_range = GRID_SIZES[0] / 2;
    spawn_systems(&mut commands, &asset_server, grid_range);
    println!("warming up for {WARMUP_SECS} seconds...");
}

fn spawn_systems(commands: &mut Commands, asset_server: &AssetServer, grid_range: i32) {
    for x in -grid_range..=grid_range {
        for y in -grid_range..=grid_range {
            commands.spawn((
                BenchParticleSystem,
                Particles3d(asset_server.load("3d-explosion.ron")),
                Transform::from_xyz(x as f32 * SPACING, y as f32 * SPACING, 0.0),
            ));
        }
    }
}

fn restart_systems(
    time: Res<Time>,
    mut timer: ResMut<RestartTimer>,
    mut emitters: Query<&mut EmitterRuntime>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        for mut emitter in &mut emitters {
            emitter.restart(None);
        }
    }
}

fn bench_tick(
    mut commands: Commands,
    mut state: ResMut<BenchState>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    systems: Query<Entity, With<BenchParticleSystem>>,
    mut exit: MessageWriter<AppExit>,
) {
    let size = GRID_SIZES[state.config_index];
    let system_count = (size * size) as u32;

    if !state.measuring {
        if state.warmup_timer.tick(time.delta()).just_finished() {
            state.measuring = true;
            state.frame = 0;
            state.total = Duration::ZERO;
            println!("measuring {system_count} systems...");
        }
        return;
    }

    state.total += time.delta();
    state.frame += 1;

    if state.frame >= MEASURE_FRAMES {
        let avg = state.total / MEASURE_FRAMES;
        state.results.push((system_count, avg));

        state.config_index += 1;
        if state.config_index >= GRID_SIZES.len() {
            print_results(&state.results);
            exit.write(AppExit::Success);
            return;
        }

        for entity in &systems {
            commands.entity(entity).despawn();
        }
        let next_size = GRID_SIZES[state.config_index];
        spawn_systems(&mut commands, &asset_server, next_size / 2);
        println!("measuring {} systems...", next_size * next_size);

        state.frame = 0;
        state.total = Duration::ZERO;
    }
}

fn print_results(results: &[(u32, Duration)]) {
    println!();
    for (systems, avg) in results {
        let us = avg.as_secs_f64() * 1_000_000.0;
        let fps = 1.0 / avg.as_secs_f64();
        println!("{systems} systems: {fps:.0} fps ({us:.2} µs/frame)");
    }
    println!();
}
