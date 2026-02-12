use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[cfg(feature = "preset-textures")]
use bevy::asset::embedded_asset;

macro_rules! preset_textures {
    ($(($variant:ident, $display:literal, $asset:literal)),* $(,)?) => {
        #[cfg(feature = "preset-textures")]
        #[derive(Debug, Clone, Serialize, Deserialize, Reflect, Hash, PartialEq, Eq)]
        pub enum PresetTexture {
            $($variant,)*
        }

        #[cfg(feature = "preset-textures")]
        impl PresetTexture {
            pub fn all() -> &'static [PresetTexture] {
                &[$(Self::$variant,)*]
            }

            pub fn display_name(&self) -> &'static str {
                match self {
                    $(Self::$variant => $display,)*
                }
            }

            pub fn embedded_path(&self) -> &'static str {
                match self {
                    $(Self::$variant => concat!("embedded://bevy_sprinkles/textures/", $asset),)*
                }
            }
        }

        #[cfg(feature = "preset-textures")]
        pub fn register_preset_textures(app: &mut App) {
            $(embedded_asset!(app, $asset);)*
        }
    };
}

preset_textures!(
    (Circle1, "Circle 1", "assets/circle_01.png"),
    (Circle2, "Circle 2", "assets/circle_02.png"),
    (Circle3, "Circle 3", "assets/circle_03.png"),
    (Circle4, "Circle 4", "assets/circle_04.png"),
    (Circle5, "Circle 5", "assets/circle_05.png"),
    (Dirt1, "Dirt 1", "assets/dirt_01.png"),
    (Dirt2, "Dirt 2", "assets/dirt_02.png"),
    (Dirt3, "Dirt 3", "assets/dirt_03.png"),
    (Fire1, "Fire 1", "assets/fire_01.png"),
    (Fire2, "Fire 2", "assets/fire_02.png"),
    (Flame1, "Flame 1", "assets/flame_01.png"),
    (Flame2, "Flame 2", "assets/flame_02.png"),
    (Flame3, "Flame 3", "assets/flame_03.png"),
    (Flame4, "Flame 4", "assets/flame_04.png"),
    (Flame5, "Flame 5", "assets/flame_05.png"),
    (Flame6, "Flame 6", "assets/flame_06.png"),
    (Flare1, "Flare 1", "assets/flare_01.png"),
    (Light1, "Light 1", "assets/light_01.png"),
    (Light2, "Light 2", "assets/light_02.png"),
    (Light3, "Light 3", "assets/light_03.png"),
    (Magic1, "Magic 1", "assets/magic_01.png"),
    (Magic2, "Magic 2", "assets/magic_02.png"),
    (Magic3, "Magic 3", "assets/magic_03.png"),
    (Magic4, "Magic 4", "assets/magic_04.png"),
    (Magic5, "Magic 5", "assets/magic_05.png"),
    (Muzzle1, "Muzzle 1", "assets/muzzle_01.png"),
    (Muzzle2, "Muzzle 2", "assets/muzzle_02.png"),
    (Muzzle3, "Muzzle 3", "assets/muzzle_03.png"),
    (Muzzle4, "Muzzle 4", "assets/muzzle_04.png"),
    (Muzzle5, "Muzzle 5", "assets/muzzle_05.png"),
    (Scorch1, "Scorch 1", "assets/scorch_01.png"),
    (Scorch2, "Scorch 2", "assets/scorch_02.png"),
    (Scorch3, "Scorch 3", "assets/scorch_03.png"),
    (Scratch1, "Scratch 1", "assets/scratch_01.png"),
    (Slash1, "Slash 1", "assets/slash_01.png"),
    (Slash2, "Slash 2", "assets/slash_02.png"),
    (Slash3, "Slash 3", "assets/slash_03.png"),
    (Slash4, "Slash 4", "assets/slash_04.png"),
    (Smoke1, "Smoke 1", "assets/smoke_01.png"),
    (Smoke2, "Smoke 2", "assets/smoke_02.png"),
    (Smoke3, "Smoke 3", "assets/smoke_03.png"),
    (Smoke4, "Smoke 4", "assets/smoke_04.png"),
    (Smoke5, "Smoke 5", "assets/smoke_05.png"),
    (Smoke6, "Smoke 6", "assets/smoke_06.png"),
    (Smoke7, "Smoke 7", "assets/smoke_07.png"),
    (Smoke8, "Smoke 8", "assets/smoke_08.png"),
    (Smoke9, "Smoke 9", "assets/smoke_09.png"),
    (Smoke10, "Smoke 10", "assets/smoke_10.png"),
    (Spark1, "Spark 1", "assets/spark_01.png"),
    (Spark2, "Spark 2", "assets/spark_02.png"),
    (Spark3, "Spark 3", "assets/spark_03.png"),
    (Spark4, "Spark 4", "assets/spark_04.png"),
    (Spark5, "Spark 5", "assets/spark_05.png"),
    (Spark6, "Spark 6", "assets/spark_06.png"),
    (Spark7, "Spark 7", "assets/spark_07.png"),
    (Star1, "Star 1", "assets/star_01.png"),
    (Star2, "Star 2", "assets/star_02.png"),
    (Star3, "Star 3", "assets/star_03.png"),
    (Star4, "Star 4", "assets/star_04.png"),
    (Star5, "Star 5", "assets/star_05.png"),
    (Star6, "Star 6", "assets/star_06.png"),
    (Star7, "Star 7", "assets/star_07.png"),
    (Star8, "Star 8", "assets/star_08.png"),
    (Star9, "Star 9", "assets/star_09.png"),
    (Symbol1, "Symbol 1", "assets/symbol_01.png"),
    (Symbol2, "Symbol 2", "assets/symbol_02.png"),
    (Trace1, "Trace 1", "assets/trace_01.png"),
    (Trace2, "Trace 2", "assets/trace_02.png"),
    (Trace3, "Trace 3", "assets/trace_03.png"),
    (Trace4, "Trace 4", "assets/trace_04.png"),
    (Trace5, "Trace 5", "assets/trace_05.png"),
    (Trace6, "Trace 6", "assets/trace_06.png"),
    (Trace7, "Trace 7", "assets/trace_07.png"),
    (Twirl1, "Twirl 1", "assets/twirl_01.png"),
    (Twirl2, "Twirl 2", "assets/twirl_02.png"),
    (Twirl3, "Twirl 3", "assets/twirl_03.png"),
    (Window1, "Window 1", "assets/window_01.png"),
    (Window2, "Window 2", "assets/window_02.png"),
    (Window3, "Window 3", "assets/window_03.png"),
    (Window4, "Window 4", "assets/window_04.png"),
);

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Hash, PartialEq, Eq)]
pub enum TextureRef {
    #[cfg(feature = "preset-textures")]
    Preset(PresetTexture),
    Asset(String),
    Local(String),
}

impl TextureRef {
    pub fn load(&self, asset_server: &AssetServer) -> Handle<Image> {
        match self {
            #[cfg(feature = "preset-textures")]
            Self::Preset(preset) => asset_server.load(preset.embedded_path()),
            Self::Local(path) | Self::Asset(path) if !path.is_empty() => asset_server.load(path),
            _ => Handle::default(),
        }
    }
}
