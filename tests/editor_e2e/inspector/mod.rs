mod scalars;
mod vectors;
mod enums;
mod complex;
mod roundtrip;

use bevy::math::Vec3;
use bevy::reflect::GetPath;

pub(super) fn read_f32<T: GetPath>(target: &T, path: &str) -> f32 {
    *target.path::<f32>(path).unwrap()
}

pub(super) fn write_f32<T: GetPath>(target: &mut T, path: &str, value: f32) {
    *target.path_mut::<f32>(path).unwrap() = value;
}

pub(super) fn read_bool<T: GetPath>(target: &T, path: &str) -> bool {
    *target.path::<bool>(path).unwrap()
}

pub(super) fn write_bool<T: GetPath>(target: &mut T, path: &str, value: bool) {
    *target.path_mut::<bool>(path).unwrap() = value;
}

pub(super) fn read_u32<T: GetPath>(target: &T, path: &str) -> u32 {
    *target.path::<u32>(path).unwrap()
}

pub(super) fn write_u32<T: GetPath>(target: &mut T, path: &str, value: u32) {
    *target.path_mut::<u32>(path).unwrap() = value;
}

pub(super) fn read_vec3<T: GetPath>(target: &T, path: &str) -> Vec3 {
    *target.path::<Vec3>(path).unwrap()
}

pub(super) fn write_vec3<T: GetPath>(target: &mut T, path: &str, value: Vec3) {
    *target.path_mut::<Vec3>(path).unwrap() = value;
}
