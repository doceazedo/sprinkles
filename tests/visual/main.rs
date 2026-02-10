// visual regression tests for particle systems.
// uses headless GPU rendering to capture frames and compare against baseline images.
// if no baseline exists, the captured frame is saved as the new baseline.
//
// run with: cargo test --test visual
// note: requires GPU support.

#[path = "../helpers/mod.rs"]
mod helpers;

use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use bevy::{
    app::{AppExit, ScheduleRunnerPlugin},
    camera::RenderTarget,
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel},
        render_resource::{
            Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode,
            PollType, TexelCopyBufferInfo, TexelCopyBufferLayout, TextureFormat, TextureUsages,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::GpuImage,
        Extract, Render, RenderApp, RenderSystems,
    },
    window::ExitCondition,
    winit::WinitPlugin,
};
use crossbeam_channel::{Receiver, Sender};

use helpers::*;
use sprinkles::runtime::ParticleSystem3D;

// ---------------------------------------------------------------------------
// headless frame capture infrastructure (adapted from bevy headless_renderer)
// ---------------------------------------------------------------------------

const CAPTURE_WIDTH: u32 = 400;
const CAPTURE_HEIGHT: u32 = 300;
const PRE_ROLL_FRAMES: u32 = 20;
const BASELINE_TOLERANCE_RATIO: f64 = 0.15;
const BASELINE_TOLERANCE_AVG_DIFF: f64 = 12.0;
const PER_CHANNEL_TOLERANCE: u8 = 20;

#[derive(Resource, Deref)]
struct MainWorldReceiver(Receiver<Vec<u8>>);

#[derive(Resource, Deref)]
struct RenderWorldSender(Sender<Vec<u8>>);

// shared buffer for extracting captured frame data out of app.run()
#[derive(Resource, Clone)]
struct CapturedFrameOutput(Arc<Mutex<Option<Vec<u8>>>>);

#[derive(Resource)]
struct CaptureConfig {
    target_frame: u32,
    current_frame: u32,
    width: u32,
    height: u32,
    fixture: String,
    system_spawned: bool,
}

struct ImageCopyPlugin;

impl Plugin for ImageCopyPlugin {
    fn build(&self, app: &mut App) {
        let (s, r) = crossbeam_channel::unbounded();

        let render_app = app
            .insert_resource(MainWorldReceiver(r))
            .sub_app_mut(RenderApp);

        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(ImageCopyLabel, ImageCopyDriver);
        graph.add_node_edge(bevy::render::graph::CameraDriverLabel, ImageCopyLabel);

        render_app
            .insert_resource(RenderWorldSender(s))
            .add_systems(ExtractSchedule, image_copy_extract)
            .add_systems(
                Render,
                receive_image_from_buffer.after(RenderSystems::Render),
            );
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, RenderLabel)]
struct ImageCopyLabel;

#[derive(Default)]
struct ImageCopyDriver;

#[derive(Clone, Default, Resource, Deref, DerefMut)]
struct ImageCopiers(pub Vec<ImageCopier>);

#[derive(Clone, Component)]
struct ImageCopier {
    buffer: Buffer,
    enabled: Arc<AtomicBool>,
    src_image: Handle<Image>,
}

impl ImageCopier {
    pub fn new(src_image: Handle<Image>, size: Extent3d, render_device: &RenderDevice) -> Self {
        let padded_bytes_per_row =
            RenderDevice::align_copy_bytes_per_row((size.width) as usize) * 4;
        let cpu_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: padded_bytes_per_row as u64 * size.height as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            buffer: cpu_buffer,
            src_image,
            enabled: Arc::new(AtomicBool::new(true)),
        }
    }
}

fn image_copy_extract(mut commands: Commands, image_copiers: Extract<Query<&ImageCopier>>) {
    commands.insert_resource(ImageCopiers(
        image_copiers.iter().cloned().collect::<Vec<ImageCopier>>(),
    ));
}

impl render_graph::Node for ImageCopyDriver {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let Some(image_copiers) = world.get_resource::<ImageCopiers>() else {
            return Ok(());
        };
        let Some(gpu_images) = world.get_resource::<RenderAssets<GpuImage>>() else {
            return Ok(());
        };

        for image_copier in image_copiers.iter() {
            if !image_copier.enabled.load(Ordering::Relaxed) {
                continue;
            }

            let Some(src_image) = gpu_images.get(&image_copier.src_image) else {
                continue;
            };

            let mut encoder = render_context
                .render_device()
                .create_command_encoder(&CommandEncoderDescriptor::default());

            let block_dimensions = src_image.texture_format.block_dimensions();
            let block_size = src_image.texture_format.block_copy_size(None).unwrap();
            let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
                (src_image.size.width as usize / block_dimensions.0 as usize)
                    * block_size as usize,
            );

            encoder.copy_texture_to_buffer(
                src_image.texture.as_image_copy(),
                TexelCopyBufferInfo {
                    buffer: &image_copier.buffer,
                    layout: TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZero::<u32>::new(padded_bytes_per_row as u32)
                                .unwrap()
                                .into(),
                        ),
                        rows_per_image: None,
                    },
                },
                src_image.size,
            );

            let render_queue = world.get_resource::<RenderQueue>().unwrap();
            render_queue.submit(std::iter::once(encoder.finish()));
        }
        Ok(())
    }
}

fn receive_image_from_buffer(
    image_copiers: Res<ImageCopiers>,
    render_device: Res<RenderDevice>,
    sender: Res<RenderWorldSender>,
) {
    for image_copier in image_copiers.0.iter() {
        if !image_copier.enabled.load(Ordering::Relaxed) {
            continue;
        }

        let buffer_slice = image_copier.buffer.slice(..);
        let (s, r) = crossbeam_channel::bounded(1);

        buffer_slice.map_async(MapMode::Read, move |r| match r {
            Ok(r) => s.send(r).expect("failed to send map update"),
            Err(err) => panic!("failed to map buffer {err}"),
        });

        render_device
            .poll(PollType::wait_indefinitely())
            .expect("failed to poll device");

        r.recv().expect("failed to receive map_async message");

        let _ = sender.send(buffer_slice.get_mapped_range().to_vec());
        image_copier.buffer.unmap();
    }
}

// ---------------------------------------------------------------------------
// scene setup & frame capture orchestration (runs inside app.run())
// ---------------------------------------------------------------------------

fn setup_scene(
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

fn spawn_particle_system(
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

fn capture_orchestrator(
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

// ---------------------------------------------------------------------------
// png image file i/o
// ---------------------------------------------------------------------------

fn save_png(path: &Path, data: &[u8], width: u32, height: u32) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let img = image::RgbaImage::from_raw(width, height, data.to_vec())
        .expect("failed to create image buffer");
    img.save(path).expect("failed to save png");
}

fn load_png(path: &Path) -> Vec<u8> {
    let img = image::open(path).expect("failed to load png");
    img.to_rgba8().into_raw()
}

// ---------------------------------------------------------------------------
// test infrastructure
// ---------------------------------------------------------------------------

fn capture_frame(fixture: &str, target_frame: u32) -> Option<Vec<u8>> {
    let output = CapturedFrameOutput(Arc::new(Mutex::new(None)));
    let output_clone = output.clone();

    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(bevy::asset::AssetPlugin {
                file_path: fixtures_path(),
                ..default()
            })
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .disable::<WinitPlugin>(),
    );

    app.add_plugins(ScheduleRunnerPlugin::run_loop(
        Duration::from_millis(1),
    ));

    app.add_plugins(sprinkles::SprinklesPlugin);
    app.add_plugins(ImageCopyPlugin);

    app.insert_resource(CaptureConfig {
        target_frame,
        current_frame: 0,
        width: CAPTURE_WIDTH,
        height: CAPTURE_HEIGHT,
        fixture: fixture.to_string(),
        system_spawned: false,
    });
    app.insert_resource(output);

    app.add_systems(Startup, setup_scene);
    app.add_systems(Update, (spawn_particle_system, capture_orchestrator));

    app.run();

    output_clone.0.lock().unwrap().take()
}

fn compare_or_generate(test_name: &str, frame_data: &[u8]) {
    let baseline_dir = screenshots_baseline_path();
    let tmp_dir = screenshots_tmp_path();
    let baseline_path = baseline_dir.join(format!("{test_name}.png"));
    let tmp_path = tmp_dir.join(format!("{test_name}.png"));

    save_png(&tmp_path, frame_data, CAPTURE_WIDTH, CAPTURE_HEIGHT);

    if !baseline_path.exists() {
        save_png(&baseline_path, frame_data, CAPTURE_WIDTH, CAPTURE_HEIGHT);
        println!("  [generated baseline: {}]", baseline_path.display());
        return;
    }

    let baseline_data = load_png(&baseline_path);
    assert_eq!(
        frame_data.len(),
        baseline_data.len(),
        "frame size mismatch for {test_name}"
    );

    let diff = compare_images(frame_data, &baseline_data, PER_CHANNEL_TOLERANCE);
    let ratio = diff.different_pixels as f64 / diff.total_pixels as f64;

    assert!(
        diff.within_tolerance(BASELINE_TOLERANCE_RATIO, BASELINE_TOLERANCE_AVG_DIFF),
        "visual regression for '{test_name}': {:.2}% pixels differ (max {:.0}%), \
         avg diff {:.2} (max {:.0}), max channel diff {}",
        ratio * 100.0,
        BASELINE_TOLERANCE_RATIO * 100.0,
        diff.avg_diff,
        BASELINE_TOLERANCE_AVG_DIFF,
        diff.max_channel_diff,
    );
}

// ---------------------------------------------------------------------------
// test definitions
// ---------------------------------------------------------------------------

fn capture_and_compare(name: &str, fixture: &str, frame: u32) {
    let data = capture_frame(fixture, frame).expect("failed to capture frame");
    compare_or_generate(name, &data);
}

fn test_fixed_seed_determinism() {
    let data_a = capture_frame("fixed_seed.ron", 30).expect("failed to capture frame (run a)");
    let data_b = capture_frame("fixed_seed.ron", 30).expect("failed to capture frame (run b)");
    let diff = compare_images(&data_a, &data_b, PER_CHANNEL_TOLERANCE);
    let ratio = diff.different_pixels as f64 / diff.total_pixels as f64;
    assert!(
        diff.within_tolerance(BASELINE_TOLERANCE_RATIO, BASELINE_TOLERANCE_AVG_DIFF),
        "fixed_seed runs differ too much: {:.2}% pixels differ, avg diff {:.2}, max channel diff {}",
        ratio * 100.0,
        diff.avg_diff,
        diff.max_channel_diff,
    );
}

// ---------------------------------------------------------------------------
// test runner (harness = false)
// ---------------------------------------------------------------------------

fn main() {
    if std::env::var("RUN_VISUAL_TESTS").is_err() {
        println!("visual regression tests are ignored. set RUN_VISUAL_TESTS=1 to run them.");
        return;
    }

    let snapshot_tests: Vec<(&str, &str, u32)> = vec![
        ("fountain_frame_30", "visual_reference_fountain.ron", 30),
        ("fountain_frame_60", "visual_reference_fountain.ron", 60),
        ("explosion_frame_10", "visual_reference_explosion.ron", 10),
        ("explosion_frame_30", "visual_reference_explosion.ron", 30),
        ("emission_shape_point", "minimal_particle_system.ron", 30),
        ("emission_shape_sphere", "all_emission_shapes.ron", 30),
        ("multiple_emitters", "two_emitters.ron", 30),
        ("color_over_lifetime", "gradients_test.ron", 30),
        ("scale_curves", "curves_test.ron", 30),
        ("collision", "collision_test.ron", 30),
        ("sub_emitter", "sub_emitter_test.ron", 30),
        ("maximal_emitter", "maximal_emitter.ron", 30),
        ("disabled_emitter", "disabled_emitter.ron", 30),
        ("fixed_fps", "fixed_fps.ron", 30),
        ("one_shot", "one_shot.ron", 20),
    ];

    let tests: Vec<(&str, Box<dyn Fn()>)> = snapshot_tests
        .into_iter()
        .map(|(name, fixture, frame)| {
            let fixture = fixture.to_string();
            let name_owned = name.to_string();
            let test_fn: Box<dyn Fn()> =
                Box::new(move || capture_and_compare(&name_owned, &fixture, frame));
            (name, test_fn)
        })
        .chain(std::iter::once((
            "fixed_seed_determinism",
            Box::new(test_fixed_seed_determinism) as Box<dyn Fn()>,
        )))
        .collect();

    let args: Vec<String> = std::env::args().collect();
    let filter = args.get(1).map(|s| s.as_str());

    let filtered: Vec<_> = tests
        .into_iter()
        .filter(|(name, _)| filter.is_none_or(|f| name.contains(f)))
        .collect();

    if filtered.is_empty() {
        println!("no visual tests matched filter");
        return;
    }

    println!("\nrunning {} visual regression test(s)\n", filtered.len());

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut failed_names = Vec::new();

    for (name, test_fn) in &filtered {
        print!("test {name} ... ");
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| test_fn())) {
            Ok(_) => {
                println!("ok");
                passed += 1;
            }
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "unknown panic".to_string()
                };
                println!("FAILED\n  {msg}");
                failed += 1;
                failed_names.push(*name);
            }
        }
    }

    println!(
        "\ntest result: {}. {passed} passed; {failed} failed\n",
        if failed == 0 { "ok" } else { "FAILED" }
    );

    if !failed_names.is_empty() {
        println!("failures:");
        for name in &failed_names {
            println!("    {name}");
        }
        println!();
        std::process::exit(1);
    }
}
