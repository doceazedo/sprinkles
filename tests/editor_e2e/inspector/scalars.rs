use super::*;

use bevy::reflect::GetPath;
use sprinkles::asset::EmitterData;

#[test]
fn test_inspect_f32_lifetime() {
    let mut emitter = EmitterData::default();
    assert_eq!(read_f32(&emitter, "time.lifetime"), 1.0);

    write_f32(&mut emitter, "time.lifetime", 3.0);
    assert_eq!(emitter.time.lifetime, 3.0);
}

#[test]
fn test_inspect_f32_noise_strength() {
    let mut emitter = EmitterData::default();
    assert_eq!(read_f32(&emitter, "turbulence.noise_strength"), 1.0);

    write_f32(&mut emitter, "turbulence.noise_strength", 2.0);
    assert_eq!(emitter.turbulence.noise_strength, 2.0);
}

#[test]
fn test_inspect_f32_spread() {
    let mut emitter = EmitterData::default();
    assert_eq!(read_f32(&emitter, "velocities.spread"), 45.0);

    write_f32(&mut emitter, "velocities.spread", 90.0);
    assert_eq!(emitter.velocities.spread, 90.0);
}

#[test]
fn test_inspect_f32_base_size() {
    let mut emitter = EmitterData::default();
    assert_eq!(read_f32(&emitter, "collision.base_size"), 0.01);

    write_f32(&mut emitter, "collision.base_size", 0.1);
    assert_eq!(emitter.collision.base_size, 0.1);
}

#[test]
fn test_inspect_f32_explosiveness() {
    let mut emitter = EmitterData::default();
    assert_eq!(read_f32(&emitter, "time.explosiveness"), 0.0);

    write_f32(&mut emitter, "time.explosiveness", 0.5);
    assert_eq!(emitter.time.explosiveness, 0.5);
}

#[test]
fn test_inspect_bool_one_shot() {
    let mut emitter = EmitterData::default();
    assert!(!read_bool(&emitter, "time.one_shot"));

    write_bool(&mut emitter, "time.one_shot", true);
    assert!(emitter.time.one_shot);
}

#[test]
fn test_inspect_bool_turbulence_enabled() {
    let mut emitter = EmitterData::default();
    assert!(!read_bool(&emitter, "turbulence.enabled"));

    write_bool(&mut emitter, "turbulence.enabled", true);
    assert!(emitter.turbulence.enabled);
}

#[test]
fn test_inspect_bool_use_scale() {
    let mut emitter = EmitterData::default();
    assert!(!read_bool(&emitter, "collision.use_scale"));

    write_bool(&mut emitter, "collision.use_scale", true);
    assert!(emitter.collision.use_scale);
}

#[test]
fn test_inspect_u32_particles_amount() {
    let mut emitter = EmitterData::default();
    assert_eq!(read_u32(&emitter, "emission.particles_amount"), 8);

    write_u32(&mut emitter, "emission.particles_amount", 100);
    assert_eq!(emitter.emission.particles_amount, 100);
}

#[test]
fn test_inspect_u32_fixed_fps() {
    let mut emitter = EmitterData::default();
    assert_eq!(read_u32(&emitter, "time.fixed_fps"), 0);

    write_u32(&mut emitter, "time.fixed_fps", 60);
    assert_eq!(emitter.time.fixed_fps, 60);
}

#[test]
fn test_inspect_optional_fixed_seed() {
    let mut emitter = EmitterData::default();
    assert!(emitter.time.fixed_seed.is_none());

    emitter.time.fixed_seed = Some(42);
    assert_eq!(emitter.time.fixed_seed, Some(42));

    let path_result = emitter.reflect_path("time.fixed_seed");
    assert!(path_result.is_ok(), "reflection path should resolve");
}
