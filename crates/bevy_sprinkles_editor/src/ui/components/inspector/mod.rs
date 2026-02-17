mod accelerations;
mod angle;
mod collider_properties;
mod collision;
mod colors;
mod draw_pass;
mod emission;
mod particle_flags;
mod scale;
mod sub_emitter;
mod time;
mod turbulence;
pub mod types;
pub mod utils;
mod velocities;

pub use types::{ComboBoxOption, FieldKind, VariantField};
pub use utils::{name_to_label, path_to_label};

use bevy::prelude::*;
use bevy_sprinkles::prelude::*;

use crate::state::{EditorState, Inspectable};
use crate::ui::icons::{ICON_BOX, ICON_SHOWERS};
use crate::ui::tokens::{
    BORDER_COLOR, FONT_PATH, TEXT_BODY_COLOR, TEXT_MUTED_COLOR, TEXT_SIZE_LG, TEXT_SIZE_SM,
};
use crate::ui::widgets::checkbox::{CheckboxProps, checkbox};
use crate::ui::widgets::combobox::{ComboBoxOptionData, combobox_with_selected};
use crate::ui::widgets::inspector_field::{InspectorFieldProps, fields_row, spawn_inspector_field};
use crate::ui::widgets::panel::{PanelDirection, PanelProps, panel};
use crate::ui::widgets::scroll::scrollbar;
use crate::ui::widgets::panel_section::{PanelSectionProps, PanelSectionSize, panel_section};
use crate::ui::widgets::variant_edit::{VariantEditProps, variant_edit};

use super::binding::FieldBinding;

pub fn plugin(app: &mut App) {
    app.init_resource::<InspectedEmitterTracker>()
        .init_resource::<InspectedColliderTracker>()
        .add_plugins((
            super::binding::plugin,
            time::plugin,
            emission::plugin,
            draw_pass::plugin,
            scale::plugin,
            angle::plugin,
            colors::plugin,
            velocities::plugin,
            accelerations::plugin,
            turbulence::plugin,
            collision::plugin,
            sub_emitter::plugin,
            particle_flags::plugin,
            collider_properties::plugin,
        ))
        .add_systems(
            Update,
            (
                (
                    update_inspected_emitter_tracker,
                    update_inspected_collider_tracker,
                ),
                (
                    cleanup_dynamic_sections,
                    setup_inspector_panel,
                    update_panel_title,
                    setup_inspector_section_fields,
                    toggle_inspector_content,
                )
                    .after(update_inspected_emitter_tracker)
                    .after(update_inspected_collider_tracker),
            ),
        );
}

#[derive(Resource, Default)]
pub struct InspectedEmitterTracker {
    pub current_index: Option<u8>,
}

#[derive(Resource, Default)]
pub struct InspectedColliderTracker {
    pub current_index: Option<u8>,
}

pub(super) fn update_inspected_emitter_tracker(
    editor_state: Res<EditorState>,
    mut tracker: ResMut<InspectedEmitterTracker>,
) {
    let new_index = editor_state
        .inspecting
        .as_ref()
        .filter(|i| i.kind == Inspectable::Emitter)
        .map(|i| i.index);

    if tracker.current_index != new_index {
        tracker.current_index = new_index;
    } else if editor_state.is_changed() {
        tracker.set_changed();
    }
}

pub(super) fn update_inspected_collider_tracker(
    editor_state: Res<EditorState>,
    mut tracker: ResMut<InspectedColliderTracker>,
) {
    let new_index = editor_state
        .inspecting
        .as_ref()
        .filter(|i| i.kind == Inspectable::Collider)
        .map(|i| i.index);

    if tracker.current_index != new_index {
        tracker.current_index = new_index;
    } else if editor_state.is_changed() {
        tracker.set_changed();
    }
}

#[derive(Component)]
pub struct EditorInspectorPanel;

#[derive(Component)]
struct InspectorPanelContent;

#[derive(Component)]
struct EmitterInspectorContent;

#[derive(Component)]
struct ColliderInspectorContent;

#[derive(Component)]
struct PanelTitleText;

#[derive(Component)]
struct PanelTitleIcon;

#[derive(Component)]
struct EnabledCheckbox;

#[derive(Component)]
pub(super) struct DynamicSectionContent;

pub fn inspector_panel(_asset_server: &AssetServer) -> impl Bundle {
    (
        EditorInspectorPanel,
        panel(
            PanelProps::new(PanelDirection::Left)
                .with_width(320)
                .with_min_width(320)
                .with_max_width(512),
        ),
    )
}

fn setup_inspector_panel(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    panels: Query<Entity, Added<EditorInspectorPanel>>,
) {
    for panel_entity in &panels {
        commands
            .entity(panel_entity)
            .with_child(scrollbar(panel_entity))
            .with_children(|parent| {
                parent.spawn(panel_title(&asset_server));

                parent
                    .spawn((
                        InspectorPanelContent,
                        Node {
                            width: percent(100),
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                    ))
                    .with_children(|content| {
                        content
                            .spawn((
                                EmitterInspectorContent,
                                Node {
                                    width: percent(100),
                                    flex_direction: FlexDirection::Column,
                                    ..default()
                                },
                            ))
                            .with_children(|emitter_content| {
                                emitter_content.spawn(time::time_section(&asset_server));
                                emitter_content.spawn(draw_pass::draw_pass_section(&asset_server));
                                emitter_content.spawn(emission::emission_section(&asset_server));
                                emitter_content.spawn(scale::scale_section(&asset_server));
                                emitter_content.spawn(colors::colors_section(&asset_server));
                                emitter_content
                                    .spawn(velocities::velocities_section(&asset_server));
                                emitter_content.spawn(angle::angle_section(&asset_server));
                                emitter_content
                                    .spawn(accelerations::accelerations_section(&asset_server));
                                emitter_content
                                    .spawn(turbulence::turbulence_section(&asset_server));
                                emitter_content.spawn(collision::collision_section(&asset_server));
                                emitter_content
                                    .spawn(sub_emitter::sub_emitter_section(&asset_server));
                                emitter_content
                                    .spawn(particle_flags::particle_flags_section(&asset_server));
                            });

                        content
                            .spawn((
                                ColliderInspectorContent,
                                Node {
                                    width: percent(100),
                                    flex_direction: FlexDirection::Column,
                                    display: Display::None,
                                    ..default()
                                },
                            ))
                            .with_children(|collider_content| {
                                collider_content.spawn(
                                    collider_properties::collider_properties_section(&asset_server),
                                );
                            });
                    });
            });
    }
}

fn toggle_inspector_content(
    editor_state: Res<EditorState>,
    mut emitter_content: Query<
        &mut Node,
        (
            With<EmitterInspectorContent>,
            Without<ColliderInspectorContent>,
            Without<EnabledCheckbox>,
        ),
    >,
    mut collider_content: Query<
        &mut Node,
        (
            With<ColliderInspectorContent>,
            Without<EmitterInspectorContent>,
            Without<EnabledCheckbox>,
        ),
    >,
    mut enabled_checkbox: Query<
        &mut Node,
        (
            With<EnabledCheckbox>,
            Without<EmitterInspectorContent>,
            Without<ColliderInspectorContent>,
        ),
    >,
) {
    if !editor_state.is_changed() {
        return;
    }

    let inspecting_kind = editor_state.inspecting.as_ref().map(|i| i.kind);

    let emitter_display = if inspecting_kind == Some(Inspectable::Emitter) {
        Display::Flex
    } else {
        Display::None
    };

    let collider_display = if inspecting_kind == Some(Inspectable::Collider) {
        Display::Flex
    } else {
        Display::None
    };

    for mut node in &mut emitter_content {
        if node.display != emitter_display {
            node.display = emitter_display;
        }
    }

    for mut node in &mut collider_content {
        if node.display != collider_display {
            node.display = collider_display;
        }
    }

    let checkbox_display = if inspecting_kind.is_some() {
        Display::Flex
    } else {
        Display::None
    };

    for mut node in &mut enabled_checkbox {
        if node.display != checkbox_display {
            node.display = checkbox_display;
        }
    }
}

fn panel_title(asset_server: &AssetServer) -> impl Bundle {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    (
        Node {
            width: percent(100),
            align_items: AlignItems::Center,
            column_gap: px(12.0),
            padding: UiRect::axes(px(24.0), px(20.0)),
            border: UiRect::bottom(px(1.0)),
            ..default()
        },
        BorderColor::all(BORDER_COLOR),
        children![
            (
                Node {
                    align_items: AlignItems::Center,
                    column_gap: px(6.0),
                    flex_grow: 1.0,
                    ..default()
                },
                children![
                    (
                        PanelTitleIcon,
                        ImageNode::new(asset_server.load(ICON_SHOWERS))
                            .with_color(Color::Srgba(TEXT_BODY_COLOR)),
                        Node {
                            width: px(16.0),
                            height: px(16.0),
                            ..default()
                        },
                    ),
                    (
                        PanelTitleText,
                        Text::new(""),
                        TextFont {
                            font: font.into(),
                            font_size: TEXT_SIZE_LG,
                            weight: FontWeight::SEMIBOLD,
                            ..default()
                        },
                        TextColor(TEXT_BODY_COLOR.into()),
                    ),
                ],
            ),
            (
                EnabledCheckbox,
                FieldBinding::emitter("enabled", FieldKind::Bool),
                checkbox(CheckboxProps::new("Enabled").checked(true), asset_server)
            ),
        ],
    )
}

pub enum InspectorItem {
    Field(InspectorFieldProps),
    Variant {
        path: String,
        props: VariantEditProps,
    },
}

impl From<InspectorFieldProps> for InspectorItem {
    fn from(props: InspectorFieldProps) -> Self {
        Self::Field(props)
    }
}

#[derive(Component)]
pub struct InspectorSection {
    pub title: String,
    pub rows: Vec<Vec<InspectorItem>>,
    initialized: bool,
}

impl InspectorSection {
    pub fn new(title: impl Into<String>, rows: Vec<Vec<InspectorItem>>) -> Self {
        Self {
            title: title.into(),
            rows,
            initialized: false,
        }
    }
}

pub(super) fn section_needs_setup<S: Component, C: Component>(
    sections: &Query<(Entity, &InspectorSection), With<S>>,
    existing: &Query<Entity, With<C>>,
) -> Option<Entity> {
    let Ok((entity, section)) = sections.single() else {
        return None;
    };
    if !section.initialized || !existing.is_empty() {
        return None;
    }
    Some(entity)
}

pub fn inspector_section(section: InspectorSection, asset_server: &AssetServer) -> impl Bundle {
    let title = section.title.clone();
    (
        section,
        panel_section(
            PanelSectionProps::new(title)
                .collapsible()
                .with_size(PanelSectionSize::XL),
            asset_server,
        ),
    )
}

fn setup_inspector_section_fields(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut sections: Query<(Entity, &mut InspectorSection)>,
) {
    for (entity, mut section) in &mut sections {
        if section.initialized {
            continue;
        }
        section.initialized = true;

        let rows = std::mem::take(&mut section.rows);

        commands.entity(entity).with_children(|parent| {
            for row_items in rows {
                parent.spawn(fields_row()).with_children(|row| {
                    for item in row_items {
                        match item {
                            InspectorItem::Field(props) => {
                                spawn_inspector_field(row, props, &asset_server);
                            }
                            InspectorItem::Variant { path, props } => {
                                row.spawn((
                                    FieldBinding::emitter(&path, FieldKind::default()),
                                    variant_edit(props),
                                ));
                            }
                        }
                    }
                });
            }
        });
    }
}

fn update_panel_title(
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    mut title_text: Query<&mut Text, With<PanelTitleText>>,
    mut title_icon: Query<&mut ImageNode, With<PanelTitleIcon>>,
    asset_server: Res<AssetServer>,
    new_titles: Query<Entity, Added<PanelTitleText>>,
) {
    let should_update = editor_state.is_changed() || !new_titles.is_empty();
    if !should_update {
        return;
    }

    let Some(inspecting) = &editor_state.inspecting else {
        return;
    };

    let Some(handle) = &editor_state.current_project else {
        return;
    };

    let Some(asset) = assets.get(handle) else {
        return;
    };

    let (name, icon_path) = match inspecting.kind {
        Inspectable::Emitter => {
            let emitter = asset.emitters.get(inspecting.index as usize);
            let name = emitter.map(|e| e.name.clone()).unwrap_or_default();
            (name, ICON_SHOWERS)
        }
        Inspectable::Collider => {
            let collider = asset.colliders.get(inspecting.index as usize);
            let name = collider.map(|c| c.name.clone()).unwrap_or_default();
            (name, ICON_BOX)
        }
    };

    for mut text in &mut title_text {
        **text = name.clone();
    }

    for mut icon in &mut title_icon {
        icon.image = asset_server.load(icon_path);
    }
}

fn cleanup_dynamic_sections(
    mut commands: Commands,
    emitter_tracker: Res<InspectedEmitterTracker>,
    collider_tracker: Res<InspectedColliderTracker>,
    existing: Query<Entity, With<DynamicSectionContent>>,
) {
    if !emitter_tracker.is_changed() && !collider_tracker.is_changed() {
        return;
    }

    for entity in &existing {
        commands.entity(entity).try_despawn();
    }
}

pub(super) fn spawn_labeled_combobox(
    parent: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    options: Vec<ComboBoxOptionData>,
    selected: usize,
    marker: impl Bundle,
) {
    parent.spawn(fields_row()).with_children(|row| {
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: Val::Px(0.0),
            ..default()
        })
        .with_children(|wrapper| {
            wrapper.spawn((
                Text::new(label),
                TextFont {
                    font: font.clone(),
                    font_size: TEXT_SIZE_SM,
                    weight: FontWeight::MEDIUM,
                    ..default()
                },
                TextColor(TEXT_MUTED_COLOR.into()),
            ));
            wrapper.spawn((marker, combobox_with_selected(options, selected)));
        });
    });
}
