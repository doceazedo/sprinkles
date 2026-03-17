use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use bevy::render::storage::ShaderStorageBuffer;
use bevy_sprinkles::material::ParticleMaterialExtension;
use bevy_sprinkles::prelude::*;
use bevy_sprinkles::{SprinklesCacheDiagnostics, SprinklesDebugFlags};
use sysinfo::System;

const LOG_INTERVAL_SECS: f64 = 5.0;

type ParticleMat = ExtendedMaterial<StandardMaterial, ParticleMaterialExtension>;

#[derive(Resource)]
struct DiagnosticsLogger {
    log_path: PathBuf,
    timer: f64,
    system: System,
    frame_count: u64,
    sim_steps_accumulator: u64,
}

pub fn plugin(app: &mut App) {
    let skip_compute = std::env::var("SKIP_COMPUTE").is_ok();
    let skip_sort = std::env::var("SKIP_SORT").is_ok();

    let tag = match (skip_compute, skip_sort) {
        (true, true) => "_skip_both",
        (true, false) => "_skip_compute",
        (false, true) => "_skip_sort",
        (false, false) => "_baseline",
    };

    let unix_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let log_path = PathBuf::from(format!("logs{tag}_{unix_ts}.txt"));

    app.insert_resource(SprinklesDebugFlags {
        skip_compute_prepare: skip_compute,
        skip_sort_prepare: skip_sort,
    });

    let mut file = File::create(&log_path).expect("Failed to create diagnostics log file");
    writeln!(
        file,
        "timestamp_s\tfps\tframe_ms\trss_mb\tentities\temitters\tcolliders\tmesh_cache\tgradient_cache\tcurve_cache\tasset_meshes\tasset_images\tasset_ssbos\tasset_materials\tavg_sim_steps\tskip_compute\tskip_sort"
    )
    .ok();

    info!(
        "Diagnostics logging to: {} (SKIP_COMPUTE={}, SKIP_SORT={})",
        log_path.display(),
        skip_compute,
        skip_sort
    );

    let mut system = System::new();
    system.refresh_memory();

    app.insert_resource(DiagnosticsLogger {
        log_path,
        timer: 0.0,
        system,
        frame_count: 0,
        sim_steps_accumulator: 0,
    });
    app.add_systems(PostUpdate, (accumulate_sim_steps, log_diagnostics).chain());
}

fn accumulate_sim_steps(
    mut logger: ResMut<DiagnosticsLogger>,
    emitters: Query<&EmitterRuntime>,
) {
    let total: usize = emitters.iter().map(|r| r.simulation_steps.len()).sum();
    logger.sim_steps_accumulator += total as u64;
    logger.frame_count += 1;
}

fn log_diagnostics(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut logger: ResMut<DiagnosticsLogger>,
    entities: Query<Entity>,
    emitters: Query<Entity, With<EmitterEntity>>,
    colliders: Query<Entity, With<ColliderEntity>>,
    cache_diag: Res<SprinklesCacheDiagnostics>,
    debug_flags: Res<SprinklesDebugFlags>,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    ssbos: Res<Assets<ShaderStorageBuffer>>,
    materials: Res<Assets<ParticleMat>>,
) {
    logger.timer += time.delta_secs_f64();
    if logger.timer < LOG_INTERVAL_SECS {
        return;
    }
    logger.timer -= LOG_INTERVAL_SECS;

    let elapsed = time.elapsed_secs_f64();

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let pid = sysinfo::get_current_pid().ok();
    logger.system.refresh_processes(
        sysinfo::ProcessesToUpdate::Some(&pid.into_iter().collect::<Vec<_>>()),
        true,
    );
    let rss_mb = pid
        .and_then(|pid| logger.system.process(pid))
        .map(|p| p.memory() as f64 / (1024.0 * 1024.0))
        .unwrap_or(0.0);

    let entity_count = entities.iter().count();
    let emitter_count = emitters.iter().count();
    let collider_count = colliders.iter().count();

    let avg_sim_steps = if logger.frame_count > 0 {
        logger.sim_steps_accumulator as f64 / logger.frame_count as f64
    } else {
        0.0
    };
    logger.sim_steps_accumulator = 0;
    logger.frame_count = 0;

    let asset_meshes = meshes.len();
    let asset_images = images.len();
    let asset_ssbos = ssbos.len();
    let asset_materials = materials.len();

    let Ok(mut file) = OpenOptions::new().append(true).open(&logger.log_path) else {
        return;
    };

    writeln!(
        file,
        "{elapsed:.1}\t{fps:.1}\t{frame_ms:.2}\t{rss_mb:.1}\t{entity_count}\t{emitter_count}\t{collider_count}\t{}\t{}\t{}\t{asset_meshes}\t{asset_images}\t{asset_ssbos}\t{asset_materials}\t{avg_sim_steps:.1}\t{}\t{}",
        cache_diag.mesh_cache_len,
        cache_diag.gradient_cache_len,
        cache_diag.curve_cache_len,
        debug_flags.skip_compute_prepare,
        debug_flags.skip_sort_prepare,
    )
    .ok();
}
