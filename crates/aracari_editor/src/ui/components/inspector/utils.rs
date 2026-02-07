use std::collections::HashMap;

use bevy::prelude::*;
use bevy::reflect::{Typed, TypeInfo, VariantInfo};
use inflector::Inflector;

use crate::ui::widgets::combobox::ComboBoxOptionData;
use crate::ui::widgets::variant_edit::VariantDefinition;
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::types::FieldDef;

const UPPERCASE_ACRONYMS: &[&str] = &["fps", "x", "y", "z"];

pub fn name_to_label(name: &str) -> String {
    let sentence = name.to_sentence_case();

    sentence
        .split_whitespace()
        .map(|word| {
            let lower = word.to_lowercase();
            if UPPERCASE_ACRONYMS.contains(&lower.as_str()) {
                lower.to_uppercase()
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn path_to_label(path: &str) -> String {
    let field_name = path.split('.').last().unwrap_or(path);
    name_to_label(field_name)
}

pub struct VariantConfig {
    pub icon: Option<&'static str>,
    pub field_overrides: Vec<(&'static str, FieldDef)>,
    pub suffix_overrides: Vec<(&'static str, VectorSuffixes)>,
    pub row_layout: Option<Vec<Vec<&'static str>>>,
    pub default_value: Option<Box<dyn PartialReflect>>,
    pub inner_struct_fields: Vec<(String, Option<FieldDef>)>,
}

impl Default for VariantConfig {
    fn default() -> Self {
        Self {
            icon: None,
            field_overrides: Vec::new(),
            suffix_overrides: Vec::new(),
            row_layout: None,
            default_value: None,
            inner_struct_fields: Vec::new(),
        }
    }
}

impl VariantConfig {
    pub fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn fields_from<T: Typed>(mut self) -> Self {
        let TypeInfo::Struct(struct_info) = T::type_info() else {
            return self;
        };

        for field in struct_info.iter() {
            let name = field.name();
            let type_path = field.type_path();
            let suffixes = self
                .suffix_overrides
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, s)| *s);

            let variant_field = field_from_type_path(name, type_path, suffixes);
            self.inner_struct_fields
                .push((name.to_string(), variant_field));
        }
        self
    }

    pub fn default_value<T: PartialReflect + Clone + 'static>(mut self, value: T) -> Self {
        self.default_value = Some(Box::new(value));
        self
    }

    pub fn override_field(mut self, name: &'static str, field: FieldDef) -> Self {
        self.field_overrides.push((name, field));
        self
    }

    pub fn override_combobox<T: Typed>(self, name: &'static str) -> Self {
        let options = combobox_options_from_reflect::<T>()
            .iter()
            .map(|o| o.label.clone())
            .collect::<Vec<_>>();
        self.override_field(name, FieldDef::combobox(name, options))
    }

    pub fn override_suffixes(mut self, name: &'static str, suffixes: VectorSuffixes) -> Self {
        self.suffix_overrides.push((name, suffixes));
        self
    }

    pub fn override_rows(mut self, layout: Vec<Vec<&'static str>>) -> Self {
        self.row_layout = Some(layout);
        self
    }
}

pub fn variants_from_reflect<T: Typed + Default + PartialReflect + Clone + 'static>(
    configs: &[(&str, VariantConfig)],
) -> Vec<VariantDefinition> {
    let TypeInfo::Enum(enum_info) = T::type_info() else {
        return Vec::new();
    };

    let config_map: HashMap<&str, &VariantConfig> =
        configs.iter().map(|(name, cfg)| (*name, cfg)).collect();

    let mut variants = Vec::new();

    for i in 0..enum_info.variant_len() {
        let Some(variant_info) = enum_info.variant_at(i) else {
            continue;
        };

        let name = variant_info.name();
        let config = config_map.get(name);

        let mut def = VariantDefinition::new(name);

        if let Some(cfg) = config {
            if let Some(icon) = cfg.icon {
                def = def.with_icon(icon);
            }

            if let Some(ref default_val) = cfg.default_value {
                match default_val.reflect_clone() {
                    Ok(cloned) => {
                        def = def.with_default_boxed(cloned.into_partial_reflect());
                    }
                    Err(err) => {
                        warn!(
                            "variants_from_reflect: reflect_clone failed for variant '{}': {:?}",
                            name, err
                        );
                    }
                }
            }
        }

        let rows = rows_from_variant_info(variant_info, config);
        if !rows.is_empty() {
            def = def.with_rows(rows);
        }

        variants.push(def);
    }

    variants
}

pub fn rows_from_variant_info(
    variant_info: &VariantInfo,
    config: Option<&&VariantConfig>,
) -> Vec<Vec<FieldDef>> {
    let override_map: HashMap<&str, &FieldDef> = config
        .map(|c| {
            c.field_overrides
                .iter()
                .map(|(name, field)| (*name, field))
                .collect()
        })
        .unwrap_or_default();

    let suffix_map: HashMap<&str, VectorSuffixes> = config
        .map(|c| {
            c.suffix_overrides
                .iter()
                .map(|(name, suffixes)| (*name, *suffixes))
                .collect()
        })
        .unwrap_or_default();

    let fields: Vec<(String, FieldDef)> = match variant_info {
        VariantInfo::Struct(struct_info) => struct_info
            .iter()
            .filter_map(|field| {
                let name = field.name();

                let variant_field = if let Some(override_field) = override_map.get(name) {
                    (*override_field).clone()
                } else {
                    let type_path = field.type_path();
                    let suffixes = suffix_map.get(name).copied();
                    field_from_type_path(name, type_path, suffixes)?
                };

                Some((name.to_string(), variant_field))
            })
            .collect(),
        VariantInfo::Tuple(_) => config
            .map(|c| {
                c.inner_struct_fields
                    .iter()
                    .filter_map(|(name, field)| {
                        if let Some(override_field) = override_map.get(name.as_str()) {
                            Some((name.clone(), (*override_field).clone()))
                        } else {
                            field.as_ref().map(|f| (name.clone(), f.clone()))
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
        VariantInfo::Unit(_) => return Vec::new(),
    };

    if let Some(cfg) = config {
        if let Some(ref layout) = cfg.row_layout {
            let fields_map: HashMap<String, FieldDef> = fields.into_iter().collect();
            return layout
                .iter()
                .map(|row_names| {
                    row_names
                        .iter()
                        .filter_map(|name| fields_map.get(*name).cloned())
                        .collect()
                })
                .filter(|row: &Vec<FieldDef>| !row.is_empty())
                .collect();
        }
    }

    fields.into_iter().map(|(_, f)| vec![f]).collect()
}

pub fn field_from_type_path(
    name: &str,
    type_path: &str,
    suffixes: Option<VectorSuffixes>,
) -> Option<FieldDef> {
    match type_path {
        "f32" => Some(FieldDef::f32(name)),
        "u32" => Some(FieldDef::u32(name)),
        "bool" => Some(FieldDef::bool(name)),
        "[f32; 4]" => Some(FieldDef::color(name)),
        path if path.contains("Gradient") && !path.contains("Interpolation") => {
            Some(FieldDef::gradient(name))
        }
        path if path.contains("Vec3") => {
            Some(FieldDef::vector(name, suffixes.unwrap_or(VectorSuffixes::XYZ)))
        }
        path if path.contains("AnimatedVelocity") => Some(FieldDef::animated_velocity(name)),
        _ => None,
    }
}

pub fn combobox_options_from_reflect<T: Typed>() -> Vec<ComboBoxOptionData> {
    let TypeInfo::Enum(enum_info) = T::type_info() else {
        return Vec::new();
    };

    (0..enum_info.variant_len())
        .filter_map(|i| {
            let variant = enum_info.variant_at(i)?;
            let name = variant.name();
            let label = name_to_label(name);
            Some(ComboBoxOptionData::new(label).with_value(name))
        })
        .collect()
}
