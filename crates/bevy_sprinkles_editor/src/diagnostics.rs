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
use bevy_sprinkles::SprinklesCacheDiagnostics;
use sysinfo::System;

const LOG_INTERVAL_SECS: f64 = 5.0;

type ParticleMat = ExtendedMaterial<StandardMaterial, ParticleMaterialExtension>;

#[derive(Resource)]
struct DiagnosticsLogger {
    log_path: PathBuf,
    timer: f64,
    system: System,
}

pub fn plugin(app: &mut App) {
    let unix_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let log_path = PathBuf::from(format!("logs_{unix_ts}.txt"));

    let mut file = File::create(&log_path).expect("Failed to create diagnostics log file");
    writeln!(
        file,
        "timestamp_s\tfps\tframe_ms\trss_mb\tentities\temitters\tcolliders\tmesh_cache\tgradient_cache\tcurve_cache\tasset_meshes\tasset_images\tasset_ssbos\tasset_materials"
    )
    .ok();

    info!("Diagnostics logging to: {}", log_path.display());

    let mut system = System::new();
    system.refresh_memory();

    app.insert_resource(DiagnosticsLogger {
        log_path,
        timer: 0.0,
        system,
    });
    app.add_systems(PostUpdate, log_diagnostics);
}

fn log_diagnostics(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut logger: ResMut<DiagnosticsLogger>,
    entities: Query<Entity>,
    emitters: Query<Entity, With<EmitterEntity>>,
    colliders: Query<Entity, With<ColliderEntity>>,
    cache_diag: Res<SprinklesCacheDiagnostics>,
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

    let asset_meshes = meshes.len();
    let asset_images = images.len();
    let asset_ssbos = ssbos.len();
    let asset_materials = materials.len();

    let Ok(mut file) = OpenOptions::new().append(true).open(&logger.log_path) else {
        return;
    };

    writeln!(
        file,
        "{elapsed:.1}\t{fps:.1}\t{frame_ms:.2}\t{rss_mb:.1}\t{entity_count}\t{emitter_count}\t{collider_count}\t{}\t{}\t{}\t{asset_meshes}\t{asset_images}\t{asset_ssbos}\t{asset_materials}",
        cache_diag.mesh_cache_len,
        cache_diag.gradient_cache_len,
        cache_diag.curve_cache_len,
    )
    .ok();
}
