![](https://raw.githubusercontent.com/doceazedo/sprinkles/main/assets/header.png)

<p align="center">
  <a href="#license">
    <img src="https://img.shields.io/badge/license-MIT%2FApache-blue.svg">
  </a>
  <a href="https://crates.io/crates/bevy_sprinkles">
    <img src="https://img.shields.io/crates/v/bevy_sprinkles.svg">
  </a>
  <a href="https://docs.rs/bevy_sprinkles/latest/bevy_sprinkles">
    <img src="https://docs.rs/bevy_sprinkles/badge.svg">
  </a>
  <a href="https://github.com/doceazedo/sprinkles/actions">
    <img src="https://github.com/doceazedo/sprinkles/workflows/CI/badge.svg">
  </a>
  <img src="https://img.shields.io/static/v1?label=Bevy&message=v0.18&color=4a6e91&logo=bevy">
</p>

# 🍩 Sprinkles

Sprinkles is a GPU-accelerated particle system for the [Bevy game engine](https://bevy.org) with a built-in dedicated visual editor.

<p align="center">
  <img src="https://raw.githubusercontent.com/doceazedo/sprinkles/main/assets/demo.gif">
</p>

## Usage

Add `bevy_sprinkles` to your project:

```toml
[dependencies]
bevy_sprinkles = "0.1"
```

Add the plugin to your Bevy app:

```rust
use bevy::prelude::*;
use bevy_sprinkles::prelude::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SprinklesPlugin))
        .run();
}
```

Spawn a particle system from a RON asset file:

```rust
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(ParticleSystem3D {
        handle: asset_server.load("my_effect.ron"),
    });
}
```

### Editor

Sprinkles comes with a visual editor for designing particle systems. To run it from the repository:

```sh
cargo editor
```

Or install the editor globally:

```sh
cargo install bevy_sprinkles_editor
```

Then run it from anywhere with the `sprinkles` command.

## Documentation

Documentation is available at [docs.rs](https://docs.rs/bevy/latest/bevy_sprinkles).

## Bevy version table

| Bevy | Sprinkles |
| ---- | --------- |
| 0.18 | 0.1, main |

## Features

- [ ] 3D
  - [ ] Emission
    - [x] Particle amount & one shot
    - [x] Explosiveness & spawn randomness
    - [x] Lifetime & lifetime randomness
    - [x] Fixed FPS & fixed seed
    - [x] Emission shapes
    - [ ] Amount ratio
    - [ ] Speed scale
    - [ ] Preprocess (simulate ahead)
    - [ ] Interpolation to end
    - [ ] Frame interpolation
    - [ ] Local coordinates
  - [ ] Direction & velocity
    - [x] Initial direction, spread & flatness
    - [x] Initial velocity
    - [x] Velocity inherit ratio & pivot
    - [x] Radial velocity
    - [x] Angular velocity
    - [ ] Orbit velocity
    - [ ] Directional velocity
    - [ ] Velocity limit curve
  - [ ] Acceleration
    - [x] Gravity
    - [ ] Linear
    - [ ] Radial
    - [ ] Tangential
    - [ ] Damping
  - [ ] Scale & rotation
    - [x] Initial scale & scale over lifetime
    - [x] Initial angle & angle over lifetime
    - [ ] Scale over velocity
  - [ ] Color
    - [x] Initial color (solid or gradient)
    - [x] Color over lifetime, alpha over lifetime, emission over lifetime
    - [ ] Hue variation
  - [x] Drawing
    - [x] Draw order
    - [x] Transform alignment
    - [x] PBR material & custom shaders
    - [x] Mesh options (quad, sphere, cuboid, cylinder, prism)
  - [x] Turbulence
  - [ ] Collision
    - [x] Collision mode, bounce, friction, scale & base size
    - [x] Box & sphere collider shapes
    - [ ] Attractors
  - [ ] Sub-emitters
  - [ ] Trails
- [ ] 2D
  - [ ] Sprite texture rendering
  - [ ] 2D trails
  - [ ] 2D visibility rect
  - [ ] Pixel-scale collision defaults

## Acknowledgements

[Godot's particle system](https://docs.godotengine.org/en/stable/tutorials/3d/particles/index.html) is a huge source of inspiration for Sprinkles, and we aim to reach feature parity at some point. Thank you for all it's contributors.

[Brackeys](https://www.youtube.com/@Brackeys)' video on making VFX with Godot was what inspired me to work on a similar system for Bevy and adapt some of those VFXs to it.

All bundled textures are provided by the very talented and generous [Kenney](https://kenney.nl/assets/particle-pack).

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/doceazedo/sprinkles/blob/main/LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](https://github.com/doceazedo/sprinkles/blob/main/LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Project examples and bundled textures are licensed under [CC0](https://creativecommons.org/publicdomain/zero/1.0/).

The editor includes two icon sets:

- Remix Icon, licensed under [Remix Icon License v1.0](https://github.com/Remix-Design/remixicon/blob/master/License)
- Blender icons, licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) by Andrzej Ambroż. <sup><small>[<a href="https://devtalk.blender.org/t/license-for-blender-icons/5522/20">source</a>]</small></sup>

The donut icon is an edited version of the Noto Emoji, licensed under [Apache 2.0](https://github.com/googlefonts/noto-emoji/blob/main/svg/LICENSE).
