use std::fs;
use std::path::Path;

use bevy::asset::embedded_asset;
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    // font
    embedded_asset!(app, "assets/InterVariable.ttf");

    // floor texture
    embedded_asset!(app, "assets/floor.png");

    // icons
    embedded_asset!(app, "assets/icons/blender-cone.png");
    embedded_asset!(app, "assets/icons/blender-cube.png");
    embedded_asset!(app, "assets/icons/blender-empty-axis.png");
    embedded_asset!(app, "assets/icons/blender-fcurve.png");
    embedded_asset!(app, "assets/icons/blender-mesh-cylinder.png");
    embedded_asset!(app, "assets/icons/blender-mesh-plane.png");
    embedded_asset!(app, "assets/icons/blender-mesh-torus.png");
    embedded_asset!(app, "assets/icons/blender-mesh-uvsphere.png");
    embedded_asset!(app, "assets/icons/blender-sphere.png");
    embedded_asset!(app, "assets/icons/blender-texture.png");
    embedded_asset!(app, "assets/icons/ri-add-line.png");
    embedded_asset!(app, "assets/icons/ri-arrow-down-s-line.png");
    embedded_asset!(app, "assets/icons/ri-arrow-left-right-fill.png");
    embedded_asset!(app, "assets/icons/ri-box-2-fill.png");
    embedded_asset!(app, "assets/icons/ri-check-fill.png");
    embedded_asset!(app, "assets/icons/ri-checkbox-circle-fill.png");
    embedded_asset!(app, "assets/icons/ri-close-circle-fill.png");
    embedded_asset!(app, "assets/icons/ri-close-fill.png");
    embedded_asset!(app, "assets/icons/ri-expand-horizontal-s-line.png");
    embedded_asset!(app, "assets/icons/ri-file-add-line.png");
    embedded_asset!(app, "assets/icons/ri-folder-image-line.png");
    embedded_asset!(app, "assets/icons/ri-folder-open-line.png");
    embedded_asset!(app, "assets/icons/ri-hashtag.png");
    embedded_asset!(app, "assets/icons/ri-heart-3-fill.png");
    embedded_asset!(app, "assets/icons/ri-information-fill.png");
    embedded_asset!(app, "assets/icons/ri-more-fill.png");
    embedded_asset!(app, "assets/icons/ri-pause-fill.png");
    embedded_asset!(app, "assets/icons/ri-play-fill.png");
    embedded_asset!(app, "assets/icons/ri-repeat-fill.png");
    embedded_asset!(app, "assets/icons/ri-seedling-fill.png");
    embedded_asset!(app, "assets/icons/ri-showers-fill.png");
    embedded_asset!(app, "assets/icons/ri-stop-fill.png");
    embedded_asset!(app, "assets/icons/ri-time-line.png");

    // shaders (except common.wgsl which uses load_internal_asset!)
    embedded_asset!(app, "assets/shaders/color_picker_alpha.wgsl");
    embedded_asset!(app, "assets/shaders/color_picker_checkerboard.wgsl");
    embedded_asset!(app, "assets/shaders/color_picker_hsv_rect.wgsl");
    embedded_asset!(app, "assets/shaders/color_picker_hue.wgsl");
    embedded_asset!(app, "assets/shaders/curve_edit.wgsl");
    embedded_asset!(app, "assets/shaders/gradient_edit.wgsl");

    // example thumbnails
    embedded_asset!(app, "assets/examples/3d-explosion.jpg");
    embedded_asset!(app, "assets/examples/acid-pool.jpg");
    embedded_asset!(app, "assets/examples/magic-puff.jpg");
    embedded_asset!(app, "assets/examples/rain.jpg");
    embedded_asset!(app, "assets/examples/windy-snow.jpg");
}

const BUNDLED_EXAMPLES: &[(&str, &str)] = &[
    (
        "3d-explosion.ron",
        include_str!("assets/examples/3d-explosion.ron"),
    ),
    (
        "acid-pool.ron",
        include_str!("assets/examples/acid-pool.ron"),
    ),
    (
        "magic-puff.ron",
        include_str!("assets/examples/magic-puff.ron"),
    ),
    ("rain.ron", include_str!("assets/examples/rain.ron")),
    (
        "windy-snow.ron",
        include_str!("assets/examples/windy-snow.ron"),
    ),
];

pub fn extract_examples(examples_dir: &Path) {
    // remove stale examples that are no longer bundled
    let bundled_names: std::collections::HashSet<&str> =
        BUNDLED_EXAMPLES.iter().map(|(name, _)| *name).collect();
    if let Ok(entries) = fs::read_dir(examples_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.ends_with(".ron") && !bundled_names.contains(name) {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    // extract (or overwrite) all bundled examples
    for (filename, contents) in BUNDLED_EXAMPLES {
        let _ = fs::write(examples_dir.join(filename), contents);
    }
}

pub fn example_thumbnail_path(stem: &str) -> String {
    format!("embedded://sprinkles/assets/examples/{stem}.jpg")
}
