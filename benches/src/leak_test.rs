use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::{Render, RenderApp};
use bevy::window::PresentMode;
use bevy_sprinkles::prelude::*;

const PHASE_DURATION_SECS: f64 = 60.0;
const LOG_INTERVAL_SECS: f64 = 5.0;

struct Phase {
    name: &'static str,
    camera: bool,
    light: bool,
    shadows: bool,
    particles: bool,
}

const PHASES: &[Phase] = &[
    Phase { name: "empty", camera: false, light: false, shadows: false, particles: false },
    Phase { name: "camera_only", camera: true, light: false, shadows: false, particles: false },
    Phase { name: "cam+light", camera: true, light: true, shadows: false, particles: false },
    Phase { name: "cam+shadows", camera: true, light: true, shadows: true, particles: false },
    Phase { name: "cam+particles", camera: true, light: true, shadows: true, particles: true },
];

#[derive(Resource)]
struct RenderEntityCounter(Arc<AtomicUsize>);

#[derive(Resource)]
struct LeakTestState {
    phase: usize,
    phase_elapsed: f64,
    log_path: PathBuf,
    log_timer: f64,
    sys: sysinfo::System,
    render_counter: Arc<AtomicUsize>,
}

#[derive(Component)]
struct PhaseCamera;

#[derive(Component)]
struct PhaseLight;

#[derive(Component)]
struct PhaseParticles;

#[derive(Resource)]
struct RestartTimer(Timer);

fn main() {
    let unix_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let log_path = PathBuf::from(format!("logs_leak_test_{unix_ts}.txt"));

    let counter = Arc::new(AtomicUsize::new(0));

    let mut file = File::create(&log_path).expect("Failed to create log file");
    writeln!(
        file,
        "timestamp_s\tphase\trss_mb\tmain_entities\trender_entities\tfps\tframe_ms"
    )
    .ok();

    let phase_names: Vec<&str> = PHASES.iter().map(|p| p.name).collect();
    println!("Leak test logging to: {}", log_path.display());
    println!(
        "Phases ({PHASE_DURATION_SECS}s each): {}",
        phase_names.join(" -> ")
    );

    let mut sys = sysinfo::System::new();
    sys.refresh_memory();

    let state = LeakTestState {
        phase: 0,
        phase_elapsed: 0.0,
        log_path,
        log_timer: 0.0,
        sys,
        render_counter: counter.clone(),
    };

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Leak Test - Phase: empty".into(),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }),
    )
    .add_plugins(SprinklesPlugin)
    .add_plugins(FrameTimeDiagnosticsPlugin::default())
    .insert_resource(state)
    .insert_resource(RestartTimer(Timer::from_seconds(3.0, TimerMode::Repeating)))
    .add_systems(
        Update,
        (
            restart_particles,
            phase_manager,
            log_diagnostics,
        )
            .chain(),
    );

    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.insert_resource(RenderEntityCounter(counter));
        render_app.add_systems(Render, count_render_entities);
    }

    app.run();
}

fn apply_phase(
    commands: &mut Commands,
    phase: &Phase,
    assets: &mut Assets<ParticleSystemAsset>,
    cameras: &Query<Entity, With<PhaseCamera>>,
    lights: &Query<Entity, With<PhaseLight>>,
    particles: &Query<Entity, With<PhaseParticles>>,
) {
    for entity in cameras.iter() {
        commands.entity(entity).despawn();
    }
    for entity in lights.iter() {
        commands.entity(entity).despawn();
    }
    for entity in particles.iter() {
        commands.entity(entity).despawn();
    }

    if phase.camera {
        commands.spawn((
            PhaseCamera,
            Camera3d::default(),
            Transform::from_xyz(5.0, 3.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
    }

    if phase.light {
        commands.spawn((
            PhaseLight,
            DirectionalLight {
                shadows_enabled: phase.shadows,
                ..default()
            },
            Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
        ));
    }

    if phase.particles {
        let emitters: Vec<EmitterData> = (0..6)
            .map(|i| EmitterData {
                name: format!("Emitter {i}"),
                emission: EmitterEmission {
                    particles_amount: 64,
                    ..default()
                },
                velocities: EmitterVelocities {
                    initial_velocity: ParticleRange::new(1.0, 5.0),
                    spread: 90.0,
                    ..default()
                },
                time: EmitterTime {
                    lifetime: 1.0,
                    one_shot: true,
                    ..default()
                },
                ..default()
            })
            .collect();

        let handle = assets.add(ParticleSystemAsset::new(
            "Test Explosion".into(),
            ParticleSystemDimension::D3,
            Default::default(),
            emitters,
            vec![],
            false,
            Default::default(),
        ));

        commands.spawn((
            PhaseParticles,
            ParticleSystem3D { handle },
            Transform::IDENTITY,
            Visibility::default(),
        ));
    }
}

fn phase_manager(
    mut commands: Commands,
    time: Res<Time>,
    mut state: ResMut<LeakTestState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    cameras: Query<Entity, With<PhaseCamera>>,
    lights: Query<Entity, With<PhaseLight>>,
    particles: Query<Entity, With<PhaseParticles>>,
    mut window: Query<&mut Window>,
    mut exit: MessageWriter<AppExit>,
) {
    state.phase_elapsed += time.delta_secs_f64();

    if state.phase_elapsed < PHASE_DURATION_SECS {
        return;
    }

    state.phase += 1;
    state.phase_elapsed = 0.0;

    if state.phase >= PHASES.len() {
        println!("[{:.0}s] All phases complete.", time.elapsed_secs());
        exit.write(AppExit::Success);
        return;
    }

    let phase = &PHASES[state.phase];
    println!("[{:.0}s] Phase {}: {}", time.elapsed_secs(), state.phase, phase.name);

    if let Ok(mut window) = window.single_mut() {
        window.title = format!("Leak Test - Phase: {}", phase.name);
    }

    apply_phase(&mut commands, phase, &mut assets, &cameras, &lights, &particles);
}

fn restart_particles(
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

fn log_diagnostics(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut state: ResMut<LeakTestState>,
    entities: Query<Entity>,
) {
    state.log_timer += time.delta_secs_f64();
    if state.log_timer < LOG_INTERVAL_SECS {
        return;
    }
    state.log_timer -= LOG_INTERVAL_SECS;

    let elapsed = time.elapsed_secs_f64();
    let phase_name = PHASES.get(state.phase).map(|p| p.name).unwrap_or("done");

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let pid = sysinfo::get_current_pid().ok();
    state.sys.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&pid.into_iter().collect::<Vec<_>>()),
        true,
    );
    let rss_mb = pid
        .and_then(|pid| state.sys.process(pid))
        .map(|p| p.memory() as f64 / (1024.0 * 1024.0))
        .unwrap_or(0.0);

    let main_entities = entities.iter().count();
    let render_entities = state.render_counter.load(Ordering::Relaxed);

    let Ok(mut file) = OpenOptions::new().append(true).open(&state.log_path) else {
        return;
    };

    writeln!(
        file,
        "{elapsed:.1}\t{phase_name}\t{rss_mb:.1}\t{main_entities}\t{render_entities}\t{fps:.1}\t{frame_ms:.2}"
    )
    .ok();
}

fn count_render_entities(entities: Query<Entity>, shared: Res<RenderEntityCounter>) {
    shared.0.store(entities.iter().count(), Ordering::Relaxed);
}
