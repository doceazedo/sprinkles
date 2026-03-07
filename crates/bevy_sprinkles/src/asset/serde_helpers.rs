use bevy::prelude::*;

pub(crate) fn is_false(v: &bool) -> bool {
    !*v
}

pub(crate) fn is_true(v: &bool) -> bool {
    *v
}

pub(crate) fn is_zero_f32(v: &f32) -> bool {
    *v == 0.0
}

pub(crate) fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

pub(crate) fn is_zero_vec2(v: &Vec2) -> bool {
    *v == Vec2::ZERO
}

pub(crate) fn is_zero_vec3(v: &Vec3) -> bool {
    *v == Vec3::ZERO
}

pub(crate) fn is_one_vec3(v: &Vec3) -> bool {
    *v == Vec3::ONE
}

pub(crate) fn is_empty_string(v: &String) -> bool {
    v.is_empty()
}
