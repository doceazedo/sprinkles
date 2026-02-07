mod accelerations;
mod collision;
mod colors;
mod draw_pass;
mod emission;
mod particle_flags;
mod scale;
mod time;
mod turbulence;
pub mod types;
pub mod utils;
mod velocities;

pub use types::{FieldKind, VariantField};
pub use utils::{name_to_label, path_to_label};

use aracari::prelude::*;
use bevy::prelude::*;

use crate::state::{EditorState, Inspectable};
use crate::ui::tokens::{BORDER_COLOR, FONT_PATH, TEXT_BODY_COLOR, TEXT_SIZE_LG};
use crate::ui::widgets::button::{ButtonVariant, IconButtonProps, icon_button};
use crate::ui::widgets::checkbox::{CheckboxProps, checkbox};
use crate::ui::widgets::inspector_field::{InspectorFieldProps, fields_row, spawn_inspector_field};
use crate::ui::widgets::panel::{PanelDirection, PanelProps, panel, panel_resize_handle, panel_scrollbar};
use crate::ui::widgets::panel_section::{PanelSectionProps, PanelSectionSize, panel_section};
use crate::ui::widgets::variant_edit::{VariantEditProps, variant_edit};

use super::binding::Field;

pub fn plugin(app: &mut App) {
    app.init_resource::<InspectedEmitterTracker>()
        .add_plugins((super::binding::plugin, time::plugin, emission::plugin, draw_pass::plugin, scale::plugin, colors::plugin, velocities::plugin, accelerations::plugin, turbulence::plugin, collision::plugin, particle_flags::plugin))
        .add_systems(Update, (
            update_inspected_emitter_tracker,
            (setup_inspector_panel, update_panel_title, setup_inspector_section_fields).after(update_inspected_emitter_tracker),
        ));
}

#[derive(Resource, Default)]
pub struct InspectedEmitterTracker {
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
    }
}

#[derive(Component)]
pub struct EditorInspectorPanel;

#[derive(Component)]
struct InspectorPanelContent;

#[derive(Component)]
struct PanelTitleText;

#[derive(Component)]
struct PanelTitleIcon;

pub fn inspector_panel(_asset_server: &AssetServer) -> impl Bundle {
    (
        EditorInspectorPanel,
        panel(
            PanelProps::new(PanelDirection::Right)
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
            .with_child(panel_resize_handle(panel_entity, PanelDirection::Right))
            .with_child(panel_scrollbar(panel_entity))
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
                        content.spawn(time::time_section(&asset_server));
                        content.spawn(draw_pass::draw_pass_section(&asset_server));
                        content.spawn(emission::emission_section(&asset_server));
                        content.spawn(scale::scale_section(&asset_server));
                        content.spawn(colors::colors_section(&asset_server));
                        content.spawn(velocities::velocities_section(&asset_server));
                        content.spawn(accelerations::accelerations_section(&asset_server));
                        content.spawn(turbulence::turbulence_section(&asset_server));
                        content.spawn(collision::collision_section(&asset_server));
                        content.spawn(particle_flags::particle_flags_section(&asset_server));
                    });
            });
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
                        ImageNode::new(asset_server.load("icons/ri-showers-fill.png"))
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
            checkbox(CheckboxProps::new("Enabled").checked(true), asset_server),
            icon_button(
                IconButtonProps::new("icons/ri-more-fill.png").variant(ButtonVariant::Ghost),
                asset_server,
            ),
        ],
    )
}

pub enum InspectorItem {
    Field(InspectorFieldProps),
    Variant { path: String, props: VariantEditProps },
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
                                row.spawn((Field::new(&path), variant_edit(props)));
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
            (name, "icons/ri-showers-fill.png")
        }
        Inspectable::Collider => ("Collider".to_string(), "icons/ri-box-2-fill.png"),
    };

    for mut text in &mut title_text {
        **text = name.clone();
    }

    for mut icon in &mut title_icon {
        icon.image = asset_server.load(icon_path);
    }
}
