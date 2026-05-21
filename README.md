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

Sprinkles is a GPU-accelerated particle system for the [Bevy game engine](https://bevy.org) with a built-in dedicated
visual editor.

<p align="center">
  <img src="https://raw.githubusercontent.com/doceazedo/sprinkles/main/assets/demo.gif">
  ⛹️ Check out all the examples <a href="./crates/bevy_sprinkles_editor/src/assets/examples">here</a>!
</p>

## Usage

Add `bevy_sprinkles` to your project:

```toml
[dependencies]
bevy_sprinkles = "0.2"
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
    commands.spawn(Particles3d {
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

Documentation is available at [docs.rs](https://docs.rs/bevy_sprinkles/latest/bevy_sprinkles/).

## Bevy version table

| Bevy | Sprinkles       |
|------|-----------------|
| 0.18 | 0.1 - 0.2, main |

## Features

- GPU particle simulation & sorting
- Fully fledged built-in visual editor
- 3D support (2D is planned)
- PBR material support (custom shaders is planned)
- Automatic GPU instancing across emitters & systems
- Particle collision
- Curl noise turbulence
- Billboard & velocity-aligned transform modes
- Sub-emitters

## Acknowledgements

[Godot's particle system](https://docs.godotengine.org/en/stable/tutorials/3d/particles/index.html) is a huge source of
inspiration for Sprinkles, and we aim to reach feature parity at some point. Thank you for all it's contributors.

[Brackeys](https://www.youtube.com/@Brackeys)' video on making VFX with Godot was what inspired me to work on a similar
system for Bevy and adapt some of those VFXs to it.

All bundled textures are provided by the very talented and generous [Kenney](https://kenney.nl/assets/particle-pack).

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/doceazedo/sprinkles/blob/main/LICENSE-APACHE)
  or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](https://github.com/doceazedo/sprinkles/blob/main/LICENSE-MIT)
  or http://opensource.org/licenses/MIT)

at your option.

Project examples and bundled textures are licensed under [CC0](https://creativecommons.org/publicdomain/zero/1.0/).

The editor includes two icon sets:

- Remix Icon, licensed under [Remix Icon License v1.0](https://github.com/Remix-Design/remixicon/blob/master/License)
- Blender icons, licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) by Andrzej
  Ambroż. <sup><small>[<a href="https://devtalk.blender.org/t/license-for-blender-icons/5522/20">source</a>]</small></sup>

The donut icon is an edited version of the Noto Emoji, licensed
under [Apache 2.0](https://github.com/googlefonts/noto-emoji/blob/main/svg/LICENSE).
