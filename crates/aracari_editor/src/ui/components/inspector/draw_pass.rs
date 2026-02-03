use aracari::prelude::*;
use bevy::prelude::*;
use bevy::reflect::{Typed, TypeInfo, VariantInfo};

use crate::ui::widgets::combobox::ComboBoxOptionData;
use crate::ui::widgets::inspector_field::InspectorFieldProps;
use crate::ui::widgets::variant_edit::{
    VariantDefinition, VariantEditProps, VariantField, name_to_label,
};
use crate::ui::widgets::vector_edit::VectorSuffixes;

use super::{InspectorItem, InspectorSection, inspector_section};

pub fn plugin(_app: &mut App) {}

pub fn draw_pass_section(asset_server: &AssetServer) -> impl Bundle {
    inspector_section(
        InspectorSection::new(
            "Draw pass",
            vec![
                vec![
                    InspectorItem::Variant {
                        path: "draw_pass.mesh".into(),
                        props: VariantEditProps::new("draw_pass.mesh")
                            .with_variants(mesh_variants()),
                    },
                    InspectorItem::Variant {
                        path: "draw_pass.material".into(),
                        props: VariantEditProps::new("draw_pass.material")
                            .with_variants(material_variants()),
                    },
                ],
                vec![InspectorFieldProps::new("draw_pass.draw_order")
                    .combobox(combobox_options_from_reflect::<DrawOrder>())
                    .into()],
                vec![InspectorFieldProps::new("draw_pass.shadow_caster")
                    .bool()
                    .into()],
            ],
        ),
        asset_server,
    )
}

struct VariantConfig {
    icon: Option<&'static str>,
    field_overrides: Vec<(&'static str, VariantField)>,
    suffix_overrides: Vec<(&'static str, VectorSuffixes)>,
    row_layout: Option<Vec<Vec<&'static str>>>,
    default_value: Option<Box<dyn PartialReflect>>,
    inner_struct_fields: Vec<(String, Option<VariantField>)>,
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
    fn icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    fn fields_from<T: Typed>(mut self) -> Self {
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

    fn default_value<T: PartialReflect + Clone + 'static>(mut self, value: T) -> Self {
        self.default_value = Some(Box::new(value));
        self
    }

    fn override_field(mut self, name: &'static str, field: VariantField) -> Self {
        self.field_overrides.push((name, field));
        self
    }

    fn override_combobox<T: Typed>(self, name: &'static str) -> Self {
        let options = combobox_options_from_reflect::<T>()
            .iter()
            .map(|o| o.label.clone())
            .collect::<Vec<_>>();
        self.override_field(name, VariantField::combobox(name, options))
    }

    fn override_suffixes(mut self, name: &'static str, suffixes: VectorSuffixes) -> Self {
        self.suffix_overrides.push((name, suffixes));
        self
    }

    fn override_rows(mut self, layout: Vec<Vec<&'static str>>) -> Self {
        self.row_layout = Some(layout);
        self
    }
}

fn variants_from_reflect<T: Typed + Default + PartialReflect + Clone + 'static>(
    configs: &[(&str, VariantConfig)],
) -> Vec<VariantDefinition> {
    let TypeInfo::Enum(enum_info) = T::type_info() else {
        return Vec::new();
    };

    let config_map: std::collections::HashMap<&str, &VariantConfig> =
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
                if let Ok(cloned) = default_val.reflect_clone() {
                    def = def.with_default_boxed(cloned.into_partial_reflect());
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

fn rows_from_variant_info(
    variant_info: &VariantInfo,
    config: Option<&&VariantConfig>,
) -> Vec<Vec<VariantField>> {
    let override_map: std::collections::HashMap<&str, &VariantField> = config
        .map(|c| {
            c.field_overrides
                .iter()
                .map(|(name, field)| (*name, field))
                .collect()
        })
        .unwrap_or_default();

    let suffix_map: std::collections::HashMap<&str, VectorSuffixes> = config
        .map(|c| {
            c.suffix_overrides
                .iter()
                .map(|(name, suffixes)| (*name, *suffixes))
                .collect()
        })
        .unwrap_or_default();

    let fields: Vec<(String, VariantField)> = match variant_info {
        VariantInfo::Struct(struct_info) => {
            struct_info
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
                .collect()
        }
        VariantInfo::Tuple(_) => {
            config
                .map(|c| {
                    c.inner_struct_fields
                        .iter()
                        .filter_map(|(name, field)| {
                            if let Some(override_field) = override_map.get(name.as_str()) {
                                Some((name.clone(), (*override_field).clone()))
                            } else {
                                field
                                    .as_ref()
                                    .map(|f| (name.clone(), f.clone()))
                            }
                        })
                        .collect()
                })
                .unwrap_or_default()
        }
        VariantInfo::Unit(_) => return Vec::new(),
    };

    if let Some(cfg) = config {
        if let Some(ref layout) = cfg.row_layout {
            let fields_map: std::collections::HashMap<String, VariantField> =
                fields.into_iter().collect();
            return layout
                .iter()
                .map(|row_names| {
                    row_names
                        .iter()
                        .filter_map(|name| fields_map.get(*name).cloned())
                        .collect()
                })
                .filter(|row: &Vec<VariantField>| !row.is_empty())
                .collect();
        }
    }

    fields.into_iter().map(|(_, f)| vec![f]).collect()
}

fn field_from_type_path(
    name: &str,
    type_path: &str,
    suffixes: Option<VectorSuffixes>,
) -> Option<VariantField> {
    match type_path {
        "f32" => Some(VariantField::f32(name)),
        "u32" => Some(VariantField::u32(name)),
        "bool" => Some(VariantField::bool(name)),
        path if path.contains("Vec3") => {
            Some(VariantField::vec3(name, suffixes.unwrap_or(VectorSuffixes::XYZ)))
        }
        _ => None,
    }
}

fn combobox_options_from_reflect<T: Typed>() -> Vec<ComboBoxOptionData> {
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

const ICON_MESH_QUAD: &str = "icons/blender_mesh_plane.png";
const ICON_MESH_SPHERE: &str = "icons/blender_mesh_uvsphere.png";
const ICON_MESH_CUBOID: &str = "icons/blender_cube.png";
const ICON_MESH_CYLINDER: &str = "icons/blender_mesh_cylinder.png";
const ICON_MESH_PRISM: &str = "icons/blender_cone.png";

fn mesh_variants() -> Vec<VariantDefinition> {
    variants_from_reflect::<ParticleMesh>(&[
        (
            "Quad",
            VariantConfig::default()
                .icon(ICON_MESH_QUAD)
                .override_combobox::<QuadOrientation>("orientation")
                .default_value(ParticleMesh::Quad {
                    orientation: QuadOrientation::default(),
                }),
        ),
        (
            "Sphere",
            VariantConfig::default()
                .icon(ICON_MESH_SPHERE)
                .default_value(ParticleMesh::Sphere { radius: 1.0 }),
        ),
        (
            "Cuboid",
            VariantConfig::default()
                .icon(ICON_MESH_CUBOID)
                .default_value(ParticleMesh::Cuboid {
                    half_size: Vec3::splat(0.5),
                }),
        ),
        (
            "Cylinder",
            VariantConfig::default()
                .icon(ICON_MESH_CYLINDER)
                .override_rows(vec![
                    vec!["top_radius", "bottom_radius"],
                    vec!["height"],
                    vec!["radial_segments", "rings"],
                    vec!["cap_top"],
                    vec!["cap_bottom"],
                ])
                .default_value(ParticleMesh::Cylinder {
                    top_radius: 0.5,
                    bottom_radius: 0.5,
                    height: 1.0,
                    radial_segments: 16,
                    rings: 1,
                    cap_top: true,
                    cap_bottom: true,
                }),
        ),
        (
            "Prism",
            VariantConfig::default()
                .icon(ICON_MESH_PRISM)
                .override_suffixes("subdivide", VectorSuffixes::WHD)
                .default_value(ParticleMesh::Prism {
                    left_to_right: 0.5,
                    size: Vec3::splat(1.0),
                    subdivide: Vec3::ZERO,
                }),
        ),
    ])
}

fn material_variants() -> Vec<VariantDefinition> {
    variants_from_reflect::<DrawPassMaterial>(&[
        (
            "Standard",
            VariantConfig::default()
                .fields_from::<StandardParticleMaterial>()
                .override_combobox::<SerializableAlphaMode>("alpha_mode")
                .default_value(DrawPassMaterial::Standard(
                    StandardParticleMaterial::default(),
                )),
        ),
        (
            "CustomShader",
            VariantConfig::default().default_value(DrawPassMaterial::CustomShader {
                vertex_shader: None,
                fragment_shader: None,
            }),
        ),
    ])
}
