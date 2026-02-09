use aracari::prelude::*;
use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::state::{DirtyState, EditorState, Inspectable, Inspecting};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonVariant, EditorButton, button, set_button_variant,
};
use crate::ui::widgets::combobox::{
    ComboBoxChangeEvent, ComboBoxPopover, ComboBoxTrigger, combobox_icon,
};
use crate::ui::widgets::dialog::{DialogActionEvent, EditorDialog, OpenConfirmationDialogEvent};
use crate::ui::widgets::panel::{PanelDirection, PanelProps, panel, panel_scrollbar};
use crate::ui::widgets::panel_section::{PanelSectionProps, panel_section};
use crate::ui::widgets::text_edit::{
    EditorTextEdit, TextEditCommitEvent, TextEditProps, text_edit,
};
use crate::viewport::{RespawnCollidersEvent, RespawnEmittersEvent};

const DOUBLE_CLICK_THRESHOLD: f32 = 0.3;

pub fn plugin(app: &mut App) {
    app.init_resource::<LastLoadedProject>()
        .add_observer(on_item_click)
        .add_observer(on_item_menu_change)
        .add_observer(on_rename_commit)
        .add_observer(on_delete_confirmed)
        .add_observer(on_add_emitter)
        .add_observer(on_add_collider)
        .add_systems(
            Update,
            (
                setup_data_panel,
                rebuild_lists,
                update_items,
                handle_item_right_click,
                handle_item_double_click,
                focus_rename_input,
                cleanup_pending_delete,
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

#[derive(Component)]
struct RenameInput {
    item_entity: Entity,
    focused: bool,
}

#[derive(Component)]
struct Renaming;

#[derive(Resource)]
struct PendingDelete {
    kind: Inspectable,
    index: u8,
}

#[derive(Event)]
struct AddEmitterEvent;

#[derive(Event)]
struct AddColliderEvent;

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
            .with_child(panel_scrollbar(panel_entity))
            .with_children(|parent| {
                parent
                    .spawn((
                        EmittersSection,
                        panel_section(
                            PanelSectionProps::new("Emitters").with_add_button(),
                            &asset_server,
                        ),
                    ))
                    .observe(on_add_emitter_click);

                parent
                    .spawn((
                        CollidersSection,
                        panel_section(
                            PanelSectionProps::new("Colliders").with_add_button(),
                            &asset_server,
                        ),
                    ))
                    .observe(on_add_collider_click);
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
        spawn_items(
            &mut commands,
            section_entity,
            Inspectable::Collider,
            asset.colliders.iter().map(|c| c.name.as_str()),
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
                combobox_icon(vec!["Duplicate", "Rename", "Delete"]),
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

fn on_add_emitter_click(_event: On<ButtonClickEvent>, mut commands: Commands) {
    commands.trigger(AddEmitterEvent);
}

fn on_add_collider_click(_event: On<ButtonClickEvent>, mut commands: Commands) {
    commands.trigger(AddColliderEvent);
}

fn on_add_emitter(
    _event: On<AddEmitterEvent>,
    mut commands: Commands,
    mut editor_state: ResMut<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut last_project: ResMut<LastLoadedProject>,
) {
    let Some(handle) = &editor_state.current_project else {
        return;
    };
    let Some(asset) = assets.get_mut(handle) else {
        return;
    };

    let existing_names: Vec<&str> = asset.emitters.iter().map(|e| e.name.as_str()).collect();
    let name = next_unique_name("Emitter", &existing_names);

    let new_index = asset.emitters.len() as u8;
    asset.emitters.push(EmitterData {
        name,
        ..Default::default()
    });

    dirty_state.has_unsaved_changes = true;

    editor_state.inspecting = Some(Inspecting {
        kind: Inspectable::Emitter,
        index: new_index,
    });

    commands.trigger(RespawnEmittersEvent);
    last_project.handle = None;
}

fn on_add_collider(
    _event: On<AddColliderEvent>,
    mut commands: Commands,
    mut editor_state: ResMut<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut last_project: ResMut<LastLoadedProject>,
) {
    let Some(handle) = &editor_state.current_project else {
        return;
    };
    let Some(asset) = assets.get_mut(handle) else {
        return;
    };

    let existing_names: Vec<&str> = asset.colliders.iter().map(|c| c.name.as_str()).collect();
    let name = next_unique_name("Collider", &existing_names);

    let new_index = asset.colliders.len() as u8;
    asset.colliders.push(ColliderData {
        name,
        ..Default::default()
    });

    dirty_state.has_unsaved_changes = true;

    editor_state.inspecting = Some(Inspecting {
        kind: Inspectable::Collider,
        index: new_index,
    });

    commands.trigger(RespawnCollidersEvent);
    last_project.handle = None;
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
    mut commands: Commands,
    mut editor_state: ResMut<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut last_project: ResMut<LastLoadedProject>,
    menus: Query<&ChildOf, With<ItemMenu>>,
    items: Query<(Entity, &InspectableItem, &Children), Without<Renaming>>,
    mut buttons: Query<&mut Node, With<ItemButton>>,
) {
    let Ok(child_of) = menus.get(event.entity) else {
        return;
    };
    let Ok((item_entity, item, children)) = items.get(child_of.parent()) else {
        return;
    };

    let item_name = get_item_name(&editor_state, &assets, item);
    let Some(item_name) = item_name else {
        return;
    };

    match event.label.as_str() {
        "Duplicate" => {
            let Some(handle) = &editor_state.current_project else {
                return;
            };
            let Some(asset) = assets.get_mut(handle) else {
                return;
            };

            let (base, _) = strip_trailing_number(&item_name);
            let insert_index = item.index as usize + 1;

            match item.kind {
                Inspectable::Emitter => {
                    let Some(source) = asset.emitters.get(item.index as usize) else {
                        return;
                    };
                    let mut new_item = source.clone();
                    let existing: Vec<&str> =
                        asset.emitters.iter().map(|e| e.name.as_str()).collect();
                    new_item.name = next_unique_name(base, &existing);
                    asset.emitters.insert(insert_index, new_item);
                }
                Inspectable::Collider => {
                    let Some(source) = asset.colliders.get(item.index as usize) else {
                        return;
                    };
                    let mut new_item = source.clone();
                    let existing: Vec<&str> =
                        asset.colliders.iter().map(|c| c.name.as_str()).collect();
                    new_item.name = next_unique_name(base, &existing);
                    asset.colliders.insert(insert_index, new_item);
                }
            }

            dirty_state.has_unsaved_changes = true;
            adjust_inspecting_after_insert(&mut editor_state.inspecting, item.kind, insert_index);
            trigger_respawn(&mut commands, item.kind);
            last_project.handle = None;
        }
        "Rename" => {
            let button_entity = children.iter().find(|c| buttons.get(*c).is_ok());
            if let Some(button_entity) = button_entity {
                if let Ok(mut btn_node) = buttons.get_mut(button_entity) {
                    btn_node.display = Display::None;
                }
            }
            start_rename(&mut commands, item_entity, &item_name);
        }
        "Delete" => {
            let label = match item.kind {
                Inspectable::Emitter => "Delete emitter",
                Inspectable::Collider => "Delete collider",
            };
            commands.insert_resource(PendingDelete {
                kind: item.kind,
                index: item.index,
            });
            commands.trigger(
                OpenConfirmationDialogEvent::new(label, "Delete")
                    .with_description(format!("Are you sure you want to delete {}?", item_name)),
            );
        }
        _ => {}
    }
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

fn next_unique_name(base_name: &str, existing: &[&str]) -> String {
    if !existing.contains(&base_name) {
        return base_name.to_string();
    }
    let mut n = 2;
    loop {
        let candidate = format!("{} {}", base_name, n);
        if !existing.iter().any(|name| *name == candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn trigger_respawn(commands: &mut Commands, kind: Inspectable) {
    match kind {
        Inspectable::Emitter => commands.trigger(RespawnEmittersEvent),
        Inspectable::Collider => commands.trigger(RespawnCollidersEvent),
    }
}

fn adjust_inspecting_after_insert(
    inspecting: &mut Option<Inspecting>,
    kind: Inspectable,
    insert_index: usize,
) {
    if let Some(current) = inspecting.as_mut() {
        if current.kind == kind && current.index as usize >= insert_index {
            current.index += 1;
        }
    }
    *inspecting = Some(Inspecting {
        kind,
        index: insert_index as u8,
    });
}

fn adjust_inspecting_after_delete(
    inspecting: &mut Option<Inspecting>,
    kind: Inspectable,
    deleted_index: usize,
    new_len: usize,
) {
    if let Some(current) = inspecting.as_ref() {
        if current.kind == kind {
            if current.index as usize == deleted_index {
                *inspecting = if new_len > 0 {
                    Some(Inspecting { kind, index: 0 })
                } else {
                    None
                };
            } else if (current.index as usize) > deleted_index {
                inspecting.as_mut().unwrap().index -= 1;
            }
        }
    }
}

fn strip_trailing_number(name: &str) -> (&str, Option<u32>) {
    if let Some(pos) = name.rfind(' ') {
        let suffix = &name[pos + 1..];
        if let Ok(n) = suffix.parse::<u32>() {
            return (name[..pos].trim_end(), Some(n));
        }
    }
    (name, None)
}

fn get_item_name(
    editor_state: &EditorState,
    assets: &Assets<ParticleSystemAsset>,
    item: &InspectableItem,
) -> Option<String> {
    let handle = editor_state.current_project.as_ref()?;
    let asset = assets.get(handle)?;
    match item.kind {
        Inspectable::Emitter => {
            let emitter = asset.emitters.get(item.index as usize)?;
            Some(emitter.name.clone())
        }
        Inspectable::Collider => {
            let collider = asset.colliders.get(item.index as usize)?;
            Some(collider.name.clone())
        }
    }
}

fn start_rename(commands: &mut Commands, item_entity: Entity, name: &str) {
    commands.entity(item_entity).insert(Renaming);

    let rename_entity = commands
        .spawn((
            RenameInput {
                item_entity,
                focused: false,
            },
            text_edit(TextEditProps::default().with_default_value(name)),
        ))
        .id();

    commands.entity(item_entity).add_child(rename_entity);
}

fn handle_item_double_click(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mouse: Res<ButtonInput<MouseButton>>,
    editor_state: Res<EditorState>,
    assets: Res<Assets<ParticleSystemAsset>>,
    items: Query<(Entity, &InspectableItem, &Children), Without<Renaming>>,
    mut buttons: Query<(Entity, &Hovered, &mut Node), With<ItemButton>>,
    mut last_click: Local<(Option<Entity>, f32)>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    for (item_entity, item, children) in &items {
        for child in children.iter() {
            let Ok((button_entity, hovered, _)) = buttons.get(child) else {
                continue;
            };
            if !hovered.get() {
                continue;
            }

            let now = time.elapsed_secs();
            let is_double = last_click.0 == Some(button_entity)
                && (now - last_click.1) < DOUBLE_CLICK_THRESHOLD;
            *last_click = (Some(button_entity), now);

            if !is_double {
                continue;
            }

            *last_click = (None, 0.0);

            let item_name = get_item_name(&editor_state, &assets, item);
            let Some(item_name) = item_name else {
                continue;
            };

            if let Ok((_, _, mut btn_node)) = buttons.get_mut(button_entity) {
                btn_node.display = Display::None;
            }

            start_rename(&mut commands, item_entity, &item_name);
            return;
        }
    }
}

fn focus_rename_input(
    mut focus: ResMut<InputFocus>,
    mut rename_inputs: Query<(Entity, &mut RenameInput)>,
    children_query: Query<&Children>,
    text_edits: Query<Entity, With<EditorTextEdit>>,
) {
    for (entity, mut rename_input) in &mut rename_inputs {
        if rename_input.focused {
            continue;
        }
        if let Some(inner) = find_inner_text_edit(entity, &children_query, &text_edits) {
            focus.0 = Some(inner);
            rename_input.focused = true;
        }
    }
}

fn find_inner_text_edit(
    entity: Entity,
    children_query: &Query<&Children>,
    text_edits: &Query<Entity, With<EditorTextEdit>>,
) -> Option<Entity> {
    if text_edits.get(entity).is_ok() {
        return Some(entity);
    }
    let Ok(children) = children_query.get(entity) else {
        return None;
    };
    for child in children.iter() {
        if let Some(found) = find_inner_text_edit(child, children_query, text_edits) {
            return Some(found);
        }
    }
    None
}

fn on_rename_commit(
    trigger: On<TextEditCommitEvent>,
    mut commands: Commands,
    rename_inputs: Query<(Entity, &RenameInput)>,
    parents: Query<&ChildOf>,
    items: Query<(&InspectableItem, &Children)>,
    mut buttons: Query<(Entity, &mut Node), With<ItemButton>>,
    mut button_texts: Query<&mut Text>,
    button_children: Query<&Children, With<EditorButton>>,
    editor_state: Res<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut emitter_runtimes: Query<&mut EmitterRuntime>,
) {
    let text_edit_entity = trigger.entity;

    let mut current = text_edit_entity;
    let mut rename_entity = None;
    for _ in 0..10 {
        if let Ok((entity, _)) = rename_inputs.get(current) {
            rename_entity = Some(entity);
            break;
        }
        let Ok(child_of) = parents.get(current) else {
            break;
        };
        current = child_of.parent();
    }

    let Some(rename_entity) = rename_entity else {
        return;
    };
    let Ok((_, rename_input)) = rename_inputs.get(rename_entity) else {
        return;
    };

    let item_entity = rename_input.item_entity;
    let new_name = trigger.text.clone();

    let Ok((item, children)) = items.get(item_entity) else {
        return;
    };

    if !new_name.is_empty() {
        if let Some(handle) = &editor_state.current_project {
            if let Some(asset) = assets.get_mut(handle) {
                match item.kind {
                    Inspectable::Emitter => {
                        if let Some(emitter) = asset.emitters.get_mut(item.index as usize) {
                            emitter.name = new_name.clone();
                            dirty_state.has_unsaved_changes = true;
                            for mut runtime in emitter_runtimes.iter_mut() {
                                runtime.restart(None);
                            }
                        }
                    }
                    Inspectable::Collider => {
                        if let Some(collider) = asset.colliders.get_mut(item.index as usize) {
                            collider.name = new_name.clone();
                            dirty_state.has_unsaved_changes = true;
                        }
                    }
                }
            }
        }
    }

    for child in children.iter() {
        if let Ok((button_entity, mut btn_node)) = buttons.get_mut(child) {
            btn_node.display = Display::Flex;

            if !new_name.is_empty() {
                if let Ok(btn_children) = button_children.get(button_entity) {
                    for btn_child in btn_children.iter() {
                        if let Ok(mut text) = button_texts.get_mut(btn_child) {
                            **text = new_name.clone();
                        }
                    }
                }
            }
        }
    }

    commands.entity(item_entity).remove::<Renaming>();
    commands.entity(rename_entity).despawn();
}

fn on_delete_confirmed(
    _event: On<DialogActionEvent>,
    pending: Option<Res<PendingDelete>>,
    mut commands: Commands,
    mut editor_state: ResMut<EditorState>,
    mut assets: ResMut<Assets<ParticleSystemAsset>>,
    mut dirty_state: ResMut<DirtyState>,
    mut last_project: ResMut<LastLoadedProject>,
) {
    let Some(pending) = pending else {
        return;
    };

    let kind = pending.kind;
    let index = pending.index as usize;
    commands.remove_resource::<PendingDelete>();

    let Some(handle) = &editor_state.current_project else {
        return;
    };
    let Some(asset) = assets.get_mut(handle) else {
        return;
    };

    let new_len = match kind {
        Inspectable::Emitter => {
            if index >= asset.emitters.len() {
                return;
            }
            asset.emitters.remove(index);
            asset.emitters.len()
        }
        Inspectable::Collider => {
            if index >= asset.colliders.len() {
                return;
            }
            asset.colliders.remove(index);
            asset.colliders.len()
        }
    };

    dirty_state.has_unsaved_changes = true;
    adjust_inspecting_after_delete(&mut editor_state.inspecting, kind, index, new_len);
    trigger_respawn(&mut commands, kind);
    last_project.handle = None;
}

fn cleanup_pending_delete(
    pending: Option<Res<PendingDelete>>,
    dialogs: Query<(), With<EditorDialog>>,
    mut commands: Commands,
) {
    if pending.is_some() && dialogs.is_empty() {
        commands.remove_resource::<PendingDelete>();
    }
}

fn update_items(
    editor_state: Res<EditorState>,
    items: Query<(&InspectableItem, &Hovered, &Children, Has<Renaming>)>,
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
    for (item, hovered, children, is_renaming) in &items {
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
                let show_menu = !is_renaming && (is_active || hovered.get() || has_open_popover);

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
