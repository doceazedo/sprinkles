use aracari::prelude::*;
use bevy::prelude::*;

use crate::state::{EditorState, Inspectable, Inspecting};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonMoreEvent, ButtonProps, ButtonVariant, EditorButton, button,
    set_button_variant,
};
use crate::ui::widgets::panel::{PanelDirection, PanelProps, panel, panel_resize_handle};
use crate::ui::widgets::panel_section::{PanelSectionProps, panel_section};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (setup_data_panel, rebuild_lists, update_button_variants));
}

#[derive(Component)]
pub struct EditorDataPanel;

#[derive(Component)]
struct EmittersSection;

#[derive(Component)]
struct CollidersSection;

#[derive(Component)]
struct InspectableItem {
    kind: Inspectable,
    index: u8,
}

pub fn data_panel(_asset_server: &AssetServer) -> impl Bundle {
    (
        EditorDataPanel,
        panel(
            PanelProps::new(PanelDirection::Left)
                .with_width(224)
                .with_min_width(160)
                .with_max_width(320),
        ),
    )
}

fn setup_data_panel(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    panels: Query<Entity, Added<EditorDataPanel>>,
) {
    for panel_entity in &panels {
        commands
            .entity(panel_entity)
            .with_child(panel_resize_handle(panel_entity, PanelDirection::Left))
            .with_children(|parent| {
                parent
                    .spawn((
                        EmittersSection,
                        panel_section(
                            PanelSectionProps::new("Emitters").with_add_button(),
                            &asset_server,
                        ),
                    ))
                    .observe(|_: On<ButtonClickEvent>| {
                        println!("TODO: add emitter");
                    });

                parent
                    .spawn((
                        CollidersSection,
                        panel_section(
                            PanelSectionProps::new("Colliders").with_add_button(),
                            &asset_server,
                        ),
                    ))
                    .observe(|_: On<ButtonClickEvent>| {
                        println!("TODO: add collider");
                    });

                parent
                    .spawn(panel_section(
                        PanelSectionProps::new("Attractors").with_add_button(),
                        &asset_server,
                    ))
                    .observe(|_: On<ButtonClickEvent>| {
                        println!("TODO: add attractor");
                    });
            });
    }
}

fn rebuild_lists(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    emitters_section: Query<(Entity, &Children), With<EmittersSection>>,
    colliders_section: Query<(Entity, &Children), With<CollidersSection>>,
    existing_items: Query<Entity, With<InspectableItem>>,
    new_sections: Query<Entity, Or<(Added<EmittersSection>, Added<CollidersSection>)>>,
) {
    let should_rebuild = editor_state.is_changed() || !new_sections.is_empty();
    if !should_rebuild {
        return;
    }

    let Some(handle) = &editor_state.current_project else {
        return;
    };

    let Some(asset) = assets.get(handle) else {
        return;
    };

    for entity in &existing_items {
        commands.entity(entity).despawn();
    }

    if let Ok((section_entity, _)) = emitters_section.single() {
        spawn_items(
            &mut commands,
            &asset_server,
            section_entity,
            Inspectable::Emitter,
            asset.emitters.iter().map(|e| e.name.as_str()),
            &editor_state,
        );
    }

    if let Ok((section_entity, _)) = colliders_section.single() {
        let empty: &[&str] = &[];
        spawn_items(
            &mut commands,
            &asset_server,
            section_entity,
            Inspectable::Collider,
            empty.iter().copied(),
            &editor_state,
        );
    }
}

fn spawn_items<'a>(
    commands: &mut Commands,
    asset_server: &AssetServer,
    section_entity: Entity,
    kind: Inspectable,
    names: impl Iterator<Item = &'a str>,
    editor_state: &EditorState,
) {
    for (index, name) in names.enumerate() {
        let index = index as u8;
        let is_active = editor_state
            .inspecting
            .map(|i| i.kind == kind && i.index == index)
            .unwrap_or(false);

        let variant = if is_active {
            ButtonVariant::Active
        } else {
            ButtonVariant::Ghost
        };

        let item_entity = commands
            .spawn((
                InspectableItem { kind, index },
                button(
                    ButtonProps::new(name)
                        .with_variant(variant)
                        .align_left()
                        .with_more(),
                    asset_server,
                ),
            ))
            .observe(on_item_click)
            .observe(on_item_more)
            .id();

        commands.entity(section_entity).add_child(item_entity);
    }
}

fn on_item_click(
    event: On<ButtonClickEvent>,
    items: Query<&InspectableItem>,
    mut editor_state: ResMut<EditorState>,
) {
    let Ok(item) = items.get(event.entity) else {
        return;
    };

    editor_state.inspecting = Some(Inspecting {
        kind: item.kind,
        index: item.index,
    });
}

fn on_item_more(_event: On<ButtonMoreEvent>, items: Query<&InspectableItem>) {
    let Ok(item) = items.get(_event.entity) else {
        return;
    };

    println!("TODO: show context menu for {:?} {}", item.kind, item.index);
}

fn update_button_variants(
    editor_state: Res<EditorState>,
    mut items: Query<
        (&InspectableItem, &mut ButtonVariant, &mut BackgroundColor, &mut BorderColor),
        With<EditorButton>,
    >,
) {
    if !editor_state.is_changed() {
        return;
    }

    for (item, mut variant, mut bg, mut border) in &mut items {
        let is_active = editor_state
            .inspecting
            .map(|i| i.kind == item.kind && i.index == item.index)
            .unwrap_or(false);

        let new_variant = if is_active {
            ButtonVariant::Active
        } else {
            ButtonVariant::Ghost
        };

        if *variant != new_variant {
            *variant = new_variant;
            set_button_variant(new_variant, &mut bg, &mut border);
        }
    }
}
