use bevy::prelude::*;
use sprinkles::prelude::*;

use crate::state::EditorState;
use crate::ui::widgets::color_picker::{CheckerboardMaterial, ColorPickerChangeEvent};
use crate::ui::widgets::gradient_edit::{EditorGradientEdit, GradientEditState, GradientMaterial};
use crate::ui::widgets::variant_edit::{
    EditorVariantEdit, VariantEditConfig, VariantEditSwatchSlot,
};

use super::{
    FieldBinding, FieldKind, MAX_ANCESTOR_DEPTH, ReflectPath, find_ancestor,
    get_inspecting_emitter,
};

#[derive(Component)]
pub(super) struct VariantSwatchOwner(Entity);

#[derive(Component)]
pub(super) struct SolidSwatchMaterial(Entity);

#[derive(Component)]
pub(super) struct GradientSwatchNode(Entity);

fn swatch_fill_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: percent(100),
        height: percent(100),
        ..default()
    }
}

fn spawn_swatch_material(
    commands: &mut Commands,
    variant_edit_entity: Entity,
    color_value: &SolidOrGradientColor,
    checkerboard_materials: &mut Assets<CheckerboardMaterial>,
    gradient_materials: &mut Assets<GradientMaterial>,
) -> Entity {
    match color_value {
        SolidOrGradientColor::Solid { color } => commands
            .spawn((
                SolidSwatchMaterial(variant_edit_entity),
                MaterialNode(checkerboard_materials.add(CheckerboardMaterial {
                    color: Vec4::new(color[0], color[1], color[2], color[3]),
                    size: 4.0,
                    border_radius: 4.0,
                })),
                swatch_fill_node(),
            ))
            .id(),
        SolidOrGradientColor::Gradient { gradient } => commands
            .spawn((
                GradientSwatchNode(variant_edit_entity),
                MaterialNode(gradient_materials.add(GradientMaterial::swatch(gradient))),
                swatch_fill_node(),
            ))
            .id(),
    }
}

pub(super) fn setup_variant_swatch(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    swatch_slots: Query<(Entity, &VariantEditSwatchSlot), Added<VariantEditSwatchSlot>>,
    variant_edit_configs: Query<(&VariantEditConfig, &FieldBinding), With<EditorVariantEdit>>,
    mut checkerboard_materials: ResMut<Assets<CheckerboardMaterial>>,
    mut gradient_materials: ResMut<Assets<GradientMaterial>>,
) {
    if swatch_slots.is_empty() {
        return;
    }

    let emitter = get_inspecting_emitter(&editor_state, &assets).map(|(_, e)| e);

    for (slot_entity, slot) in &swatch_slots {
        let variant_edit_entity = slot.0;
        let Ok((_config, binding)) = variant_edit_configs.get(variant_edit_entity) else {
            continue;
        };

        let Some(emitter) = emitter else {
            continue;
        };

        let reflect_path = ReflectPath::new(binding.path());
        let Ok(value) = emitter.reflect_path(reflect_path.as_str()) else {
            continue;
        };

        let Some(color_value) = value.try_downcast_ref::<SolidOrGradientColor>() else {
            continue;
        };

        commands
            .entity(slot_entity)
            .insert(VariantSwatchOwner(variant_edit_entity));

        let material_entity = spawn_swatch_material(
            &mut commands,
            variant_edit_entity,
            color_value,
            &mut checkerboard_materials,
            &mut gradient_materials,
        );
        commands.entity(slot_entity).add_child(material_entity);
    }
}

pub(super) fn sync_variant_swatch_from_color(
    trigger: On<ColorPickerChangeEvent>,
    field_bindings: Query<&FieldBinding>,
    solid_swatches: Query<(&SolidSwatchMaterial, &MaterialNode<CheckerboardMaterial>)>,
    mut checkerboard_materials: ResMut<Assets<CheckerboardMaterial>>,
    parents: Query<&ChildOf>,
) {
    let binding = find_ancestor(trigger.entity, &parents, MAX_ANCESTOR_DEPTH, |e| {
        field_bindings.get(e).is_ok()
    })
    .and_then(|e| field_bindings.get(e).ok());

    let Some(binding) = binding else {
        return;
    };

    if !matches!(binding.kind, FieldKind::Color) {
        return;
    }

    let Some(variant_edit) = binding.variant_edit else {
        return;
    };

    for (solid, mat_node) in &solid_swatches {
        if solid.0 != variant_edit {
            continue;
        }
        if let Some(mat) = checkerboard_materials.get_mut(&mat_node.0) {
            let c = trigger.color;
            mat.color = Vec4::new(c[0], c[1], c[2], c[3]);
        }
    }
}

pub(super) fn sync_variant_swatch_from_gradient(
    mut commands: Commands,
    gradient_edits: Query<
        (Entity, &GradientEditState, &FieldBinding),
        (With<EditorGradientEdit>, Changed<GradientEditState>),
    >,
    swatches: Query<(Entity, &VariantSwatchOwner, &Children)>,
    gradient_nodes: Query<Entity, With<GradientSwatchNode>>,
    mut gradient_materials: ResMut<Assets<GradientMaterial>>,
) {
    for (_, state, binding) in &gradient_edits {
        if !matches!(binding.kind, FieldKind::Gradient) {
            continue;
        }

        let Some(variant_edit) = binding.variant_edit else {
            continue;
        };

        let Some((swatch_entity, _, swatch_children)) = swatches
            .iter()
            .find(|(_, owner, _)| owner.0 == variant_edit)
        else {
            continue;
        };

        for child in swatch_children.iter() {
            if gradient_nodes.get(child).is_ok() {
                commands.entity(child).try_despawn();
            }
        }

        let material_entity = commands
            .spawn((
                GradientSwatchNode(variant_edit),
                MaterialNode(gradient_materials.add(GradientMaterial::swatch(&state.gradient))),
                swatch_fill_node(),
            ))
            .id();
        commands.entity(swatch_entity).add_child(material_entity);
    }
}

pub(super) fn respawn_variant_swatch_on_switch(
    mut commands: Commands,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    changed_configs: Query<(Entity, &VariantEditConfig, &FieldBinding), Changed<VariantEditConfig>>,
    swatches: Query<(Entity, &VariantSwatchOwner, &Children)>,
    mut checkerboard_materials: ResMut<Assets<CheckerboardMaterial>>,
    mut gradient_materials: ResMut<Assets<GradientMaterial>>,
) {
    let Some((_, emitter)) = get_inspecting_emitter(&editor_state, &assets) else {
        return;
    };

    for (variant_edit_entity, config, binding) in &changed_configs {
        if !config.show_swatch_slot {
            continue;
        }

        let Some((swatch_entity, _, swatch_children)) = swatches
            .iter()
            .find(|(_, owner, _)| owner.0 == variant_edit_entity)
        else {
            continue;
        };

        for child in swatch_children.iter() {
            commands.entity(child).try_despawn();
        }

        let reflect_path = ReflectPath::new(binding.path());
        let Ok(value) = emitter.reflect_path(reflect_path.as_str()) else {
            continue;
        };

        let Some(color_value) = value.try_downcast_ref::<SolidOrGradientColor>() else {
            continue;
        };

        let material_entity = spawn_swatch_material(
            &mut commands,
            variant_edit_entity,
            color_value,
            &mut checkerboard_materials,
            &mut gradient_materials,
        );
        commands.entity(swatch_entity).add_child(material_entity);
    }
}
