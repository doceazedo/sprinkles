# Roadmap

This is an unordered list of planned features. Nothing here is guaranteed to be added.

## Core

- 2D particle systems[^1]
- Add smoothstep interpolation for `GradientEdit`
- Add `editor_only` checkbox for colliders (useful if already present in-game)

### Godot feature parity

Features from [GPUParticles3D](https://docs.godotengine.org/en/stable/classes/class_gpuparticles3d.html)
and [ParticleProcessMaterial](https://docs.godotengine.org/en/stable/classes/class_particleprocessmaterial.html) not yet
implemented in Sprinkles.

- **EmitterData**
    - **EmitterTime**
        - speed_scale
        - preprocess
        - fract_delta
        - interpolate
        - interpolate_to_end
    - **EmitterDrawPass**
        - **ParticleMesh**
            - Custom GLTF
        - **DrawPassMaterial**
            - Convert it from an enum to a struct and add optional shader fields to it directly
    - **EmitterEmission**
        - amount_ratio
        - **EmissionShape**
            - Point
            - Box
            - Points
            - DirectedPoints
    - **EmitterScale**
        - scale_over_velocity
    - **EmitterColors**
        - ~~hue_variation~~[^2]
        - ~~hue_variation_over_lifetime~~[^2]
    - **EmitterVelocities**
        - velocity_limit_over_lifetime
    - **EmitterAccelerations**
        - linear_acceleration
        - radial_acceleration
        - tangential_acceleration
        - damping (+ ParticleFlags::DAMPING_AS_FRICTION
    - **EmitterTurbulence**
        - initial_displacement
    - ~~**EmitterSpritesheet**~~[^2]
    - ~~**VisibilityAabb**~~[^2]
- **AttractorData**

## Editor

### QoL

- <kbd>⌘</kbd> + <kbd>Z</kbd> (undo/redo)
- Reorder emitters and colliders via drag and drop
- Better open project error messages (too vague)
- In-editor docs
- WASM build

### Settings

- Check for updates automatically
- Grid / Floor / Skybox
- Light mode (maybe?)
- Footnotes (version, hash, links/buttons...)

## Testing

- Stress test example
- Regression tests with screenshots

## Docs

- Guide for using Sprinkles in-game

[^1]: Eventually.

[^2]: Not planned, personally.
