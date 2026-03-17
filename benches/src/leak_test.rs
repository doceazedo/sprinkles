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

const DURATION_SECS: f64 = 1800.0;
const LOG_INTERVAL_SECS: f64 = 30.0;

#[derive(Resource)]
struct RenderEntityCounter(Arc<AtomicUsize>);

#[derive(Resource)]
struct LeakTestState {
    log_path: PathBuf,
    log_timer: f64,
    sys: sysinfo::System,
    render_counter: Arc<AtomicUsize>,
    manual_frames: u64,
    manual_frames_at_last_log: u64,
    samples: Vec<LogSample>,
}

struct LogSample {
    timestamp: f64,
    rss_mb: f64,
    main_entities: usize,
    render_entities: usize,
    diag_fps: f64,
    manual_fps: f64,
}

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
    writeln!(file, "Sprinkles Leak Test — 3d-explosion.ron, {DURATION_SECS}s").ok();
    writeln!(file, "=====================================================").ok();
    writeln!(file).ok();
    writeln!(
        file,
        "{:<10} {:>10} {:>10} {:>10} {:>10} {:>8} {:>8}",
        "time", "rss_mb", "delta_mb", "main_e", "render_e", "diag_fps", "real_fps"
    )
    .ok();
    writeln!(file, "{}", "-".repeat(72)).ok();

    println!("\n  Sprinkles Leak Test — 3d-explosion.ron");
    println!("  Duration: {DURATION_SECS}s, logging every {LOG_INTERVAL_SECS}s");
    println!("  Log: {}\n", log_path.display());

    let mut sys = sysinfo::System::new();
    sys.refresh_memory();

    let state = LeakTestState {
        log_path,
        log_timer: 0.0,
        sys,
        render_counter: counter.clone(),
        manual_frames: 0,
        manual_frames_at_last_log: 0,
        samples: Vec::new(),
    };

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Leak Test — 3d-explosion.ron".into(),
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
    .add_systems(Startup, setup_scene)
    .add_systems(Update, (restart_particles, count_frames, log_diagnostics).chain());

    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.insert_resource(RenderEntityCounter(counter));
        render_app.add_systems(Render, count_render_entities);
    }

    app.run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 3.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, 0.4, 0.0)),
    ));

    commands.spawn((
        ParticleSystem3D {
            handle: asset_server.load("3d-explosion.ron"),
        },
        Transform::IDENTITY,
        Visibility::default(),
    ));
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

fn count_frames(mut state: ResMut<LeakTestState>) {
    state.manual_frames += 1;
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
    mut exit: MessageWriter<AppExit>,
) {
    state.log_timer += time.delta_secs_f64();
    if state.log_timer < LOG_INTERVAL_SECS {
        return;
    }
    state.log_timer -= LOG_INTERVAL_SECS;

    let elapsed = time.elapsed_secs_f64();

    if elapsed > DURATION_SECS {
        print_summary(&state);
        exit.write(AppExit::Success);
        return;
    }

    let diag_fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let frames_this_interval = state.manual_frames - state.manual_frames_at_last_log;
    let manual_fps = frames_this_interval as f64 / LOG_INTERVAL_SECS;
    state.manual_frames_at_last_log = state.manual_frames;

    let rss_mb = get_rss_mb(&mut state.sys);
    let main_entities = entities.iter().count();
    let render_entities = state.render_counter.load(Ordering::Relaxed);

    let first_rss = state.samples.first().map(|s| s.rss_mb).unwrap_or(rss_mb);
    let delta_mb = rss_mb - first_rss;

    state.samples.push(LogSample {
        timestamp: elapsed,
        rss_mb,
        main_entities,
        render_entities,
        diag_fps,
        manual_fps,
    });

    let line = format!(
        "{:<10.0} {:>10.1} {:>+10.1} {:>10} {:>10} {:>8.0} {:>8.0}",
        elapsed, rss_mb, delta_mb, main_entities, render_entities, diag_fps, manual_fps
    );
    println!("  {line}");

    if let Ok(mut file) = OpenOptions::new().append(true).open(&state.log_path) {
        writeln!(file, "{line}").ok();
    }
}

fn print_summary(state: &LeakTestState) {
    if state.samples.len() < 2 {
        return;
    }

    let first = &state.samples[0];
    let last = &state.samples[state.samples.len() - 1];
    let duration_min = (last.timestamp - first.timestamp) / 60.0;
    let total_growth = last.rss_mb - first.rss_mb;
    let rate = if duration_min > 0.0 { total_growth / duration_min } else { 0.0 };
    let avg_fps: f64 = state.samples.iter().map(|s| s.manual_fps).sum::<f64>()
        / state.samples.len() as f64;

    let summary = format!(
        "\n\nSummary\n=======\n\
         Duration:       {:.1} min\n\
         RSS:            {:.1} MB -> {:.1} MB ({:+.1} MB)\n\
         Growth rate:    {:.2} MB/min\n\
         Avg FPS (real): {:.0}\n\
         Per-frame leak: {:.4} KB/frame\n",
        duration_min,
        first.rss_mb, last.rss_mb, total_growth,
        rate,
        avg_fps,
        if avg_fps > 0.0 { (rate * 1024.0) / (avg_fps * 60.0) } else { 0.0 },
    );

    print!("{summary}");
    if let Ok(mut file) = OpenOptions::new().append(true).open(&state.log_path) {
        write!(file, "{summary}").ok();
    }
}

fn count_render_entities(entities: Query<Entity>, shared: Res<RenderEntityCounter>) {
    shared.0.store(entities.iter().count(), Ordering::Relaxed);
}
