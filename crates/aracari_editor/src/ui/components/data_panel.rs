use aracari::prelude::*;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::state::{EditorState, Inspectable, Inspecting};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonVariant, EditorButton, button, set_button_variant,
};
use crate::ui::widgets::combobox::{
    ComboBoxChangeEvent, ComboBoxPopover, ComboBoxTrigger, combobox_icon,
};
use crate::ui::widgets::panel::{PanelDirection, PanelProps, panel, panel_resize_handle};
use crate::ui::widgets::panel_section::{PanelSectionProps, panel_section};

pub fn plugin(app: &mut App) {
    app.init_resource::<LastLoadedProject>()
        .add_observer(on_item_click)
        .add_observer(on_item_menu_change)
        .add_systems(
            Update,
            (
                setup_data_panel,
                rebuild_lists,
                update_items,
                handle_item_right_click,
            ),
        );
}

#[derive(Resource, Default)]
struct LastLoadedProject {
    handle: Option<AssetId<ParticleSystemAsset>>,
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

#[derive(Component)]
struct ItemButton;

#[derive(Component)]
struct ItemMenu;

#[derive(Component)]
struct ItemsList;

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
    editor_state: Res<EditorState>,
    mut last_project: ResMut<LastLoadedProject>,
    assets: Res<Assets<ParticleSystemAsset>>,
    emitters_section: Query<(Entity, &Children), With<EmittersSection>>,
    colliders_section: Query<(Entity, &Children), With<CollidersSection>>,
    existing_wrappers: Query<Entity, With<ItemsList>>,
    new_sections: Query<Entity, Or<(Added<EmittersSection>, Added<CollidersSection>)>>,
) {
    let Some(handle) = &editor_state.current_project else {
        return;
    };

    let Some(asset) = assets.get(handle) else {
        return;
    };

    let current_id = handle.id();
    let project_changed = last_project.handle != Some(current_id);
    let sections_added = !new_sections.is_empty();

    if !project_changed && !sections_added {
        return;
    }

    last_project.handle = Some(current_id);

    for entity in &existing_wrappers {
        commands.entity(entity).despawn();
    }

    if let Ok((section_entity, _)) = emitters_section.single() {
        spawn_items(
            &mut commands,
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
            section_entity,
            Inspectable::Collider,
            empty.iter().copied(),
            &editor_state,
        );
    }
}

fn spawn_items<'a>(
    commands: &mut Commands,
    section_entity: Entity,
    kind: Inspectable,
    names: impl Iterator<Item = &'a str>,
    editor_state: &EditorState,
) {
    let names: Vec<_> = names.collect();
    if names.is_empty() {
        return;
    }

    let list_entity = commands
        .spawn((
            ItemsList,
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.0),
                ..default()
            },
        ))
        .id();

    commands.entity(section_entity).add_child(list_entity);

    for (index, name) in names.into_iter().enumerate() {
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
                Hovered::default(),
                Interaction::None,
                Node {
                    width: percent(100),
                    ..default()
                },
            ))
            .id();

        let button_entity = commands
            .spawn((
                ItemButton,
                button(ButtonProps::new(name).with_variant(variant).align_left()),
            ))
            .id();

        let menu_entity = commands
            .spawn((
                ItemMenu,
                combobox_icon(vec!["Option A", "Option B", "Option C"]),
            ))
            .insert(Node {
                position_type: PositionType::Absolute,
                right: px(0.0),
                top: px(0.0),
                ..default()
            })
            .id();

        commands
            .entity(item_entity)
            .add_children(&[button_entity, menu_entity]);

        commands.entity(list_entity).add_child(item_entity);
    }
}

fn on_item_click(
    event: On<ButtonClickEvent>,
    buttons: Query<&ChildOf, With<ItemButton>>,
    items: Query<&InspectableItem>,
    mut editor_state: ResMut<EditorState>,
) {
    let Ok(child_of) = buttons.get(event.entity) else {
        return;
    };
    let Ok(item) = items.get(child_of.parent()) else {
        return;
    };

    editor_state.inspecting = Some(Inspecting {
        kind: item.kind,
        index: item.index,
    });
}

fn on_item_menu_change(
    event: On<ComboBoxChangeEvent>,
    menus: Query<&ChildOf, With<ItemMenu>>,
    items: Query<&InspectableItem>,
) {
    let Ok(child_of) = menus.get(event.entity) else {
        return;
    };
    let Ok(item) = items.get(child_of.parent()) else {
        return;
    };

    println!("TODO: {} for {:?} {}", event.label, item.kind, item.index);
}

fn handle_item_right_click(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    items: Query<(&Hovered, &Children), With<InspectableItem>>,
    buttons: Query<&Hovered, With<ItemButton>>,
    menus: Query<&Children, With<ItemMenu>>,
    triggers: Query<Entity, With<ComboBoxTrigger>>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }

    for (item_hovered, item_children) in &items {
        if !item_hovered.get() {
            continue;
        }

        let mut button_hovered = false;
        let mut menu_entity = None;

        for child in item_children.iter() {
            if let Ok(btn_hovered) = buttons.get(child) {
                button_hovered = btn_hovered.get();
            }
            if menus.get(child).is_ok() {
                menu_entity = Some(child);
            }
        }

        if !button_hovered {
            continue;
        }

        let Some(menu) = menu_entity else {
            continue;
        };

        let Ok(menu_children) = menus.get(menu) else {
            continue;
        };

        for menu_child in menu_children.iter() {
            if triggers.get(menu_child).is_ok() {
                commands.trigger(ButtonClickEvent { entity: menu_child });
                return;
            }
        }
    }
}

fn update_items(
    editor_state: Res<EditorState>,
    items: Query<(&InspectableItem, &Hovered, &Children)>,
    buttons: Query<&Children, With<ItemButton>>,
    mut button_styles: Query<
        (&mut ButtonVariant, &mut BackgroundColor, &mut BorderColor),
        With<EditorButton>,
    >,
    mut menus: Query<(Entity, &mut Node, &Children), With<ItemMenu>>,
    trigger_children: Query<
        &Children,
        (
            Without<InspectableItem>,
            Without<ItemButton>,
            Without<ItemMenu>,
        ),
    >,
    mut images: Query<&mut ImageNode>,
    mut text_colors: Query<&mut TextColor>,
    popovers: Query<&ComboBoxPopover>,
) {
    for (item, hovered, children) in &items {
        let is_active = editor_state
            .inspecting
            .map(|i| i.kind == item.kind && i.index == item.index)
            .unwrap_or(false);

        let new_variant = if is_active {
            ButtonVariant::Active
        } else {
            ButtonVariant::Ghost
        };

        let text_color = new_variant.text_color();

        for child in children.iter() {
            if let Ok(button_children) = buttons.get(child) {
                if let Ok((mut variant, mut bg, mut border)) = button_styles.get_mut(child) {
                    if *variant != new_variant {
                        *variant = new_variant;
                        set_button_variant(new_variant, &mut bg, &mut border);

                        for button_child in button_children.iter() {
                            if let Ok(mut color) = text_colors.get_mut(button_child) {
                                color.0 = text_color.into();
                            }
                            if let Ok(mut image) = images.get_mut(button_child) {
                                image.color = text_color.into();
                            }
                        }
                    }
                }
            }
            if let Ok((menu_entity, mut node, menu_kids)) = menus.get_mut(child) {
                let has_open_popover = popovers.iter().any(|p| p.0 == menu_entity);
                let show_menu = is_active || hovered.get() || has_open_popover;

                node.display = if show_menu {
                    Display::Flex
                } else {
                    Display::None
                };
                for menu_child in menu_kids.iter() {
                    if let Ok(children) = trigger_children.get(menu_child) {
                        for trigger_child in children.iter() {
                            if let Ok(mut image) = images.get_mut(trigger_child) {
                                image.color = text_color.into();
                            }
                        }
                    }
                }
            }
        }
    }
}
