use std::sync::{Arc, Mutex};

use bevy::{
    app::AppExit,
    camera::RenderTarget,
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureFormat, TextureUsages},
        renderer::RenderDevice,
    },
};

use super::frame_capture::{ImageCopier, MainWorldReceiver};
use super::PRE_ROLL_FRAMES;
use sprinkles::runtime::ParticleSystem3D;

#[derive(Resource, Clone)]
pub struct CapturedFrameOutput(pub Arc<Mutex<Option<Vec<u8>>>>);

#[derive(Resource)]
pub struct CaptureConfig {
    pub target_frame: u32,
    pub current_frame: u32,
    pub width: u32,
    pub height: u32,
    pub fixture: String,
    pub system_spawned: bool,
}

pub fn setup_scene(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    render_device: Res<RenderDevice>,
    config: Res<CaptureConfig>,
) {
    let size = Extent3d {
        width: config.width,
        height: config.height,
        ..default()
    };

    let mut render_target_image =
        Image::new_target_texture(size.width, size.height, TextureFormat::bevy_default(), None);
    render_target_image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let render_target_handle = images.add(render_target_image);

    commands.spawn(ImageCopier::new(
        render_target_handle.clone(),
        size,
        &render_device,
    ));

    commands.spawn((
        Camera3d::default(),
        RenderTarget::Image(render_target_handle.into()),
        Tonemapping::None,
        Transform::from_xyz(0.0, 3.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        PointLight {
            intensity: 500_000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}

pub fn spawn_particle_system(
    mut commands: Commands,
    asset_server: Res<bevy::asset::AssetServer>,
    mut config: ResMut<CaptureConfig>,
) {
    if config.system_spawned {
        return;
    }

    let handle: Handle<sprinkles::asset::ParticleSystemAsset> =
        asset_server.load(config.fixture.clone());
    commands.spawn(ParticleSystem3D { handle });
    config.system_spawned = true;
}

pub fn capture_orchestrator(
    receiver: Res<MainWorldReceiver>,
    mut config: ResMut<CaptureConfig>,
    output: Res<CapturedFrameOutput>,
    mut app_exit: MessageWriter<AppExit>,
) {
    config.current_frame += 1;

    let total_needed = config.target_frame + PRE_ROLL_FRAMES;
    if config.current_frame < total_needed {
        // drain any premature captures
        while receiver.try_recv().is_ok() {}
        return;
    }

    // try to receive the captured frame
    let mut image_data = Vec::new();
    while let Ok(data) = receiver.try_recv() {
        image_data = data;
    }

    if !image_data.is_empty() {
        // strip row padding
        let row_bytes = config.width as usize * 4;
        let aligned_row_bytes = RenderDevice::align_copy_bytes_per_row(row_bytes);
        let final_data = if row_bytes == aligned_row_bytes {
            image_data
        } else {
            image_data
                .chunks(aligned_row_bytes)
                .take(config.height as usize)
                .flat_map(|row| &row[..row_bytes.min(row.len())])
                .cloned()
                .collect()
        };

        *output.0.lock().unwrap() = Some(final_data);
        app_exit.write(AppExit::Success);
    }

    // safety: exit after too many extra frames
    if config.current_frame > total_needed + 30 {
        app_exit.write(AppExit::Success);
    }
}
