use bevy::{
    prelude::*,
    render::{
        extract_resource::ExtractResource,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
};
use std::collections::HashMap;

use crate::asset::{Gradient, GradientInterpolation, ParticleSystemAsset, SolidOrGradientColor};
use crate::core::ParticleSystem3D;

const GRADIENT_TEXTURE_WIDTH: u32 = 256;

#[derive(Resource, Default)]
pub struct GradientTextureCache {
    cache: HashMap<u64, Handle<Image>>,
}

impl GradientTextureCache {
    pub fn get_or_create(
        &mut self,
        gradient: &Gradient,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        let key = gradient.cache_key();

        if let Some(handle) = self.cache.get(&key) {
            return handle.clone();
        }

        let image = bake_gradient_texture(gradient);
        let handle = images.add(image);
        self.cache.insert(key, handle.clone());
        handle
    }

    pub fn get(&self, gradient: &Gradient) -> Option<Handle<Image>> {
        let key = gradient.cache_key();
        self.cache.get(&key).cloned()
    }
}

fn bake_gradient_texture(gradient: &Gradient) -> Image {
    let width = GRADIENT_TEXTURE_WIDTH;
    let mut data = Vec::with_capacity((width * 4) as usize);

    for i in 0..width {
        let t = if width > 1 {
            i as f32 / (width - 1) as f32
        } else {
            0.0
        };
        let color = sample_gradient(gradient, t);

        data.push((color[0] * 255.0).clamp(0.0, 255.0) as u8);
        data.push((color[1] * 255.0).clamp(0.0, 255.0) as u8);
        data.push((color[2] * 255.0).clamp(0.0, 255.0) as u8);
        data.push((color[3] * 255.0).clamp(0.0, 255.0) as u8);
    }

    let mut image = Image::new(
        Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC;

    image
}

fn sample_gradient(gradient: &Gradient, t: f32) -> [f32; 4] {
    let stops = &gradient.stops;

    if stops.is_empty() {
        return [1.0, 1.0, 1.0, 1.0];
    }

    if stops.len() == 1 {
        return stops[0].color;
    }

    let t = t.clamp(0.0, 1.0);

    let mut left_idx = 0;
    let mut right_idx = stops.len() - 1;

    for (i, stop) in stops.iter().enumerate() {
        if stop.position <= t {
            left_idx = i;
        }
    }

    for (i, stop) in stops.iter().enumerate() {
        if stop.position >= t {
            right_idx = i;
            break;
        }
    }

    let left = &stops[left_idx];
    let right = &stops[right_idx];

    if left_idx == right_idx {
        return left.color;
    }

    let range = right.position - left.position;
    if range <= 0.0 {
        return left.color;
    }

    let local_t = (t - left.position) / range;

    match gradient.interpolation {
        GradientInterpolation::Steps => left.color,
        GradientInterpolation::Linear => lerp_color(left.color, right.color, local_t),
        GradientInterpolation::Smoothstep => {
            let smooth_t = local_t * local_t * (3.0 - 2.0 * local_t);
            lerp_color(left.color, right.color, smooth_t)
        }
    }
}

fn lerp_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

#[derive(Resource, Clone, ExtractResource)]
pub struct FallbackGradientTexture {
    pub handle: Handle<Image>,
}

pub fn create_fallback_gradient_texture(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![255, 255, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC;

    let handle = images.add(image);
    commands.insert_resource(FallbackGradientTexture { handle });
}

pub fn prepare_gradient_textures(
    mut cache: ResMut<GradientTextureCache>,
    mut images: ResMut<Assets<Image>>,
    particle_systems: Query<&ParticleSystem3D>,
    assets: Res<Assets<ParticleSystemAsset>>,
) {
    for system in &particle_systems {
        let Some(asset) = assets.get(&system.handle) else {
            continue;
        };

        for emitter in &asset.emitters {
            if let SolidOrGradientColor::Gradient { gradient } =
                &emitter.process.display.color_curves.initial_color
            {
                cache.get_or_create(gradient, &mut images);
            }
        }
    }
}
