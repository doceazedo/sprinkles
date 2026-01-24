use bevy::{
    prelude::*,
    render::{
        extract_resource::ExtractResource,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
};
use std::collections::HashMap;

use crate::asset::{Knot, ParticleSystemAsset, SplineCurve};
use crate::core::ParticleSystem3D;

const CURVE_TEXTURE_WIDTH: u32 = 256;

#[derive(Resource, Default)]
pub struct CurveTextureCache {
    cache: HashMap<u64, Handle<Image>>,
}

impl CurveTextureCache {
    pub fn get_or_create(
        &mut self,
        curve: &SplineCurve,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        let key = curve.cache_key();

        if let Some(handle) = self.cache.get(&key) {
            return handle.clone();
        }

        let image = bake_curve_texture(curve);
        let handle = images.add(image);
        self.cache.insert(key, handle.clone());
        handle
    }

    pub fn get(&self, curve: &SplineCurve) -> Option<Handle<Image>> {
        let key = curve.cache_key();
        self.cache.get(&key).cloned()
    }
}

fn bake_curve_texture(curve: &SplineCurve) -> Image {
    let width = CURVE_TEXTURE_WIDTH;
    let knots = curve.to_knots();
    let mut data = Vec::with_capacity((width * 4) as usize);

    for i in 0..width {
        let t = if width > 1 {
            i as f32 / (width - 1) as f32
        } else {
            0.0
        };
        let value = sample_knots(&knots, t);

        let byte = (value.clamp(0.0, 1.0) * 255.0) as u8;
        data.push(byte); // R
        data.push(byte); // G
        data.push(byte); // B
        data.push(255); // A
    }

    let mut image = Image::new(
        Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8Unorm,
        default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC;

    image
}

/// Samples the curve at position t by interpolating between knots.
fn sample_knots(knots: &[Knot], t: f32) -> f32 {
    if knots.is_empty() {
        return 1.0;
    }

    if knots.len() == 1 {
        return knots[0].value;
    }

    let t = t.clamp(0.0, 1.0);

    // find surrounding knots
    let mut left_idx = 0;
    let mut right_idx = knots.len() - 1;

    for (i, knot) in knots.iter().enumerate() {
        if knot.position <= t {
            left_idx = i;
        }
    }

    for (i, knot) in knots.iter().enumerate() {
        if knot.position >= t {
            right_idx = i;
            break;
        }
    }

    let left = &knots[left_idx];
    let right = &knots[right_idx];

    if left_idx == right_idx {
        return left.value;
    }

    let range = right.position - left.position;
    if range <= 0.0 {
        return left.value;
    }

    // linear interpolation between knots
    let local_t = (t - left.position) / range;
    left.value + (right.value - left.value) * local_t
}

#[derive(Resource, Clone, ExtractResource)]
pub struct FallbackCurveTexture {
    pub handle: Handle<Image>,
}

pub fn create_fallback_curve_texture(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![255, 255, 255, 255],
        TextureFormat::Rgba8Unorm,
        default(),
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::COPY_SRC;

    let handle = images.add(image);
    commands.insert_resource(FallbackCurveTexture { handle });
}

pub fn prepare_curve_textures(
    mut cache: ResMut<CurveTextureCache>,
    mut images: ResMut<Assets<Image>>,
    particle_systems: Query<&ParticleSystem3D>,
    assets: Res<Assets<ParticleSystemAsset>>,
) {
    for system in &particle_systems {
        let Some(asset) = assets.get(&system.handle) else {
            continue;
        };

        for emitter in &asset.emitters {
            if let Some(curve) = &emitter.process.display.scale.curve {
                if !curve.is_constant() {
                    cache.get_or_create(curve, &mut images);
                }
            }
            if let Some(curve) = &emitter.process.display.color_curves.alpha_curve {
                if !curve.is_constant() {
                    cache.get_or_create(curve, &mut images);
                }
            }
        }
    }
}
