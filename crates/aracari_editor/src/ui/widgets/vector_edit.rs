use crate::ui::tokens::{TEXT_SIZE, TEXT_SIZE_SM};
use crate::ui::widgets::text_edit::{TextEditPrefix, TextEditProps, text_edit};
use bevy::prelude::*;

#[derive(Component)]
pub struct EditorVectorEdit;

#[derive(Default, Clone, Copy)]
pub enum VectorSuffixes {
    #[default]
    XYZ,
    WHD,
    Range,
}

impl VectorSuffixes {
    fn get(&self, index: usize) -> &'static str {
        match self {
            Self::XYZ => ["X", "Y", "Z"].get(index).unwrap_or(&""),
            Self::WHD => ["W", "H", "D"].get(index).unwrap_or(&""),
            Self::Range => ["min", "max"].get(index).unwrap_or(&""),
        }
    }

    fn size(&self) -> f32 {
        match self {
            Self::Range => TEXT_SIZE_SM,
            _ => TEXT_SIZE,
        }
    }
}

#[derive(Default, Clone, Copy)]
pub enum VectorSize {
    Vec2,
    #[default]
    Vec3,
}

impl VectorSize {
    fn count(&self) -> usize {
        match self {
            Self::Vec2 => 2,
            Self::Vec3 => 3,
        }
    }
}

pub struct VectorEditProps {
    pub label: Option<String>,
    pub size: VectorSize,
    pub suffixes: VectorSuffixes,
    pub default_values: Vec<f32>,
}

impl Default for VectorEditProps {
    fn default() -> Self {
        Self {
            label: None,
            size: VectorSize::Vec3,
            suffixes: VectorSuffixes::XYZ,
            default_values: Vec::new(),
        }
    }
}

impl VectorEditProps {
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_size(mut self, size: VectorSize) -> Self {
        self.size = size;
        self
    }

    pub fn with_suffixes(mut self, suffixes: VectorSuffixes) -> Self {
        self.suffixes = suffixes;
        self
    }

    pub fn with_default_values(mut self, values: impl Into<Vec<f32>>) -> Self {
        self.default_values = values.into();
        self
    }

    pub fn vec2(mut self) -> Self {
        self.size = VectorSize::Vec2;
        self
    }

    pub fn vec3(mut self) -> Self {
        self.size = VectorSize::Vec3;
        self
    }
}

pub fn vector_edit(props: VectorEditProps) -> impl Bundle {
    let VectorEditProps {
        label,
        size,
        suffixes,
        default_values,
    } = props;

    let children: Vec<_> = (0..size.count())
        .map(|i| {
            let mut text_edit_props =
                TextEditProps::default()
                    .numeric_f32()
                    .with_prefix(TextEditPrefix::Label {
                        label: suffixes.get(i).to_string(),
                        size: suffixes.size(),
                    });

            if i == 0 {
                if let Some(ref label) = label {
                    text_edit_props = text_edit_props.with_label(label.clone());
                }
            }

            if let Some(&value) = default_values.get(i) {
                text_edit_props = text_edit_props.with_default_value(value.to_string());
            }

            text_edit(text_edit_props)
        })
        .collect();

    (
        EditorVectorEdit,
        Node {
            width: percent(100),
            column_gap: px(12),
            align_items: AlignItems::FlexEnd,
            ..default()
        },
        Children::spawn(SpawnIter(children.into_iter())),
    )
}
