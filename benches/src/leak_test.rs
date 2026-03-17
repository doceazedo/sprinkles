use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::light::{CascadeShadowConfig, DirectionalLightShadowMap};
use bevy::prelude::*;
use bevy::render::{Render, RenderApp};
use bevy::window::PresentMode;

const PHASE_DURATION_SECS: f64 = 60.0;
const LOG_INTERVAL_SECS: f64 = 5.0;

struct Phase {
    name: &'static str,
    description: &'static str,
    setup: fn(&mut Commands, &mut ResMut<Assets<Mesh>>, &mut ResMut<Assets<StandardMaterial>>),
}

const PHASES: &[Phase] = &[
    Phase {
        name: "no_shadows",
        description: "Camera + directional light (shadows OFF)",
        setup: setup_no_shadows,
    },
    Phase {
        name: "dir_4cascade",
        description: "Directional light, 4 cascades (default), shadow map 2048",
        setup: setup_dir_4cascade,
    },
    Phase {
        name: "dir_1cascade",
        description: "Directional light, 1 cascade, shadow map 2048",
        setup: setup_dir_1cascade,
    },
    Phase {
        name: "dir_4cascade_sm512",
        description: "Directional light, 4 cascades, shadow map 512",
        setup: setup_dir_4cascade_small_map,
    },
    Phase {
        name: "dir_4cascade_sm4096",
        description: "Directional light, 4 cascades, shadow map 4096",
        setup: setup_dir_4cascade_large_map,
    },
    Phase {
        name: "point_shadow",
        description: "Point light with shadows (6 cube faces)",
        setup: setup_point_shadow,
    },
    Phase {
        name: "spot_shadow",
        description: "Spot light with shadows",
        setup: setup_spot_shadow,
    },
    Phase {
        name: "dir_4cascade_mesh",
        description: "Directional light, 4 cascades + shadow-casting cube",
        setup: setup_dir_4cascade_with_mesh,
    },
];

#[derive(Component)]
struct PhaseEntity;

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
    phase_start_rss: f64,
    samples: Vec<LogSample>,
}

struct LogSample {
    timestamp: f64,
    phase: &'static str,
    rss_mb: f64,
    main_entities: usize,
    render_entities: usize,
    fps: f64,
}

fn main() {
    let unix_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let log_path = PathBuf::from(format!("logs_leak_test_{unix_ts}.txt"));

    let counter = Arc::new(AtomicUsize::new(0));

    let mut file = File::create(&log_path).expect("Failed to create log file");
    writeln!(file, "Sprinkles Shadow Leak Test").ok();
    writeln!(file, "==========================").ok();
    writeln!(file, "Phase duration: {PHASE_DURATION_SECS}s each").ok();
    writeln!(file).ok();
    writeln!(file, "Phases:").ok();
    for (i, phase) in PHASES.iter().enumerate() {
        writeln!(file, "  {i}. {:<24} {}", phase.name, phase.description).ok();
    }
    writeln!(file).ok();
    writeln!(
        file,
        "{:<10} {:<24} {:>10} {:>8} {:>8} {:>8}",
        "time", "phase", "rss_mb", "main_e", "render_e", "fps"
    )
    .ok();
    writeln!(file, "{}", "-".repeat(74)).ok();

    println!("\n  Sprinkles Shadow Leak Test");
    println!("  ==========================");
    println!("  Logging to: {}", log_path.display());
    println!("  Phase duration: {PHASE_DURATION_SECS}s each\n");
    for (i, phase) in PHASES.iter().enumerate() {
        println!("  {i}. {:<24} {}", phase.name, phase.description);
    }
    println!();

    let mut sys = sysinfo::System::new();
    sys.refresh_memory();

    let state = LeakTestState {
        phase: 0,
        phase_elapsed: 0.0,
        log_path,
        log_timer: 0.0,
        sys,
        render_counter: counter.clone(),
        phase_start_rss: 0.0,
        samples: Vec::new(),
    };

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Leak Test: {}", PHASES[0].name),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            }),
            ..default()
        }),
    )
    .add_plugins(FrameTimeDiagnosticsPlugin::default())
    .insert_resource(state)
    .add_systems(Startup, setup_scene)
    .add_systems(Update, (phase_manager, log_diagnostics).chain());

    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.insert_resource(RenderEntityCounter(counter));
        render_app.add_systems(Render, count_render_entities);
    }

    app.run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera3d::default());
    println!("  [{:>6.1}s] Phase 0: {} — {}", 0.0, PHASES[0].name, PHASES[0].description);
}

fn phase_manager(
    mut commands: Commands,
    time: Res<Time>,
    mut state: ResMut<LeakTestState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    phase_entities: Query<Entity, With<PhaseEntity>>,
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
        print_summary(&state);
        exit.write(AppExit::Success);
        return;
    }

    for entity in phase_entities.iter() {
        commands.entity(entity).despawn();
    }

    let phase = &PHASES[state.phase];
    println!(
        "  [{:>6.1}s] Phase {}: {} — {}",
        time.elapsed_secs(),
        state.phase,
        phase.name,
        phase.description
    );

    if let Ok(mut window) = window.single_mut() {
        window.title = format!("Leak Test: {}", phase.name);
    }

    (phase.setup)(&mut commands, &mut meshes, &mut materials);
}

fn spawn_light_transform() -> Transform {
    Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0))
}

fn setup_no_shadows(
    commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PhaseEntity,
        DirectionalLight {
            shadows_enabled: false,
            ..default()
        },
        spawn_light_transform(),
    ));
}

fn setup_dir_4cascade(
    commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PhaseEntity,
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        spawn_light_transform(),
    ));
}

fn setup_dir_1cascade(
    commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PhaseEntity,
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        spawn_light_transform(),
        CascadeShadowConfig {
            bounds: vec![10.0],
            ..default()
        },
    ));
}

fn setup_dir_4cascade_small_map(
    commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(DirectionalLightShadowMap { size: 512 });
    commands.spawn((
        PhaseEntity,
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        spawn_light_transform(),
    ));
}

fn setup_dir_4cascade_large_map(
    commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(DirectionalLightShadowMap { size: 4096 });
    commands.spawn((
        PhaseEntity,
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        spawn_light_transform(),
    ));
}

fn setup_point_shadow(
    commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(DirectionalLightShadowMap { size: 2048 });
    commands.spawn((
        PhaseEntity,
        PointLight {
            shadows_enabled: true,
            range: 20.0,
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 0.0),
    ));
}

fn setup_spot_shadow(
    commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        PhaseEntity,
        SpotLight {
            shadows_enabled: true,
            range: 20.0,
            ..default()
        },
        Transform::from_xyz(0.0, 5.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn setup_dir_4cascade_with_mesh(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(DirectionalLightShadowMap { size: 2048 });
    commands.spawn((
        PhaseEntity,
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        spawn_light_transform(),
    ));
    commands.spawn((
        PhaseEntity,
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        Transform::from_xyz(0.0, 0.0, -5.0),
    ));
}

fn get_rss_mb(sys: &mut sysinfo::System) -> f64 {
    let pid = sysinfo::get_current_pid().ok();
    sys.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&pid.into_iter().collect::<Vec<_>>()),
        true,
    );
    pid.and_then(|pid| sys.process(pid))
        .map(|p| p.memory() as f64 / (1024.0 * 1024.0))
        .unwrap_or(0.0)
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

    let rss_mb = get_rss_mb(&mut state.sys);
    let main_entities = entities.iter().count();
    let render_entities = state.render_counter.load(Ordering::Relaxed);

    if state.phase_start_rss == 0.0 {
        state.phase_start_rss = rss_mb;
    }

    state.samples.push(LogSample {
        timestamp: elapsed,
        phase: phase_name,
        rss_mb,
        main_entities,
        render_entities,
        fps,
    });

    let line = format!(
        "{:<10.1} {:<24} {:>10.1} {:>8} {:>8} {:>8.0}",
        elapsed, phase_name, rss_mb, main_entities, render_entities, fps
    );

    if let Ok(mut file) = OpenOptions::new().append(true).open(&state.log_path) {
        writeln!(file, "{line}").ok();
    }
}

fn print_summary(state: &LeakTestState) {
    let mut summary = String::new();
    summary.push_str("\n\nSummary: RSS growth rate per phase\n");
    summary.push_str("===================================\n\n");
    summary.push_str(&format!(
        "{:<24} {:>12} {:>12} {:>14} {:>10}\n",
        "phase", "start_mb", "end_mb", "growth_mb/min", "avg_fps"
    ));
    summary.push_str(&format!("{}\n", "-".repeat(76)));

    let mut current_phase = "";
    let mut phase_first_rss = 0.0;
    let mut phase_first_time = 0.0;
    let mut phase_last_rss = 0.0;
    let mut phase_last_time = 0.0;
    let mut phase_fps_sum = 0.0;
    let mut phase_fps_count = 0u32;

    for (i, sample) in state.samples.iter().enumerate() {
        let is_new_phase = sample.phase != current_phase;
        let is_last = i == state.samples.len() - 1;

        if is_new_phase && !current_phase.is_empty() {
            let duration_min = (phase_last_time - phase_first_time) / 60.0;
            let growth = phase_last_rss - phase_first_rss;
            let rate = if duration_min > 0.0 { growth / duration_min } else { 0.0 };
            let avg_fps = if phase_fps_count > 0 { phase_fps_sum / phase_fps_count as f64 } else { 0.0 };
            summary.push_str(&format!(
                "{:<24} {:>12.1} {:>12.1} {:>14.2} {:>10.0}\n",
                current_phase, phase_first_rss, phase_last_rss, rate, avg_fps
            ));
        }

        if is_new_phase {
            current_phase = sample.phase;
            phase_first_rss = sample.rss_mb;
            phase_first_time = sample.timestamp;
            phase_fps_sum = 0.0;
            phase_fps_count = 0;
        }

        phase_last_rss = sample.rss_mb;
        phase_last_time = sample.timestamp;
        phase_fps_sum += sample.fps;
        phase_fps_count += 1;

        if is_last {
            let duration_min = (phase_last_time - phase_first_time) / 60.0;
            let growth = phase_last_rss - phase_first_rss;
            let rate = if duration_min > 0.0 { growth / duration_min } else { 0.0 };
            let avg_fps = if phase_fps_count > 0 { phase_fps_sum / phase_fps_count as f64 } else { 0.0 };
            summary.push_str(&format!(
                "{:<24} {:>12.1} {:>12.1} {:>14.2} {:>10.0}\n",
                current_phase, phase_first_rss, phase_last_rss, rate, avg_fps
            ));
        }
    }

    print!("{summary}");
    if let Ok(mut file) = OpenOptions::new().append(true).open(&state.log_path) {
        write!(file, "{summary}").ok();
    }
}

fn count_render_entities(entities: Query<Entity>, shared: Res<RenderEntityCounter>) {
    shared.0.store(entities.iter().count(), Ordering::Relaxed);
}
