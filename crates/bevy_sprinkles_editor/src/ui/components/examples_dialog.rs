use bevy::color::palettes::tailwind;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use bevy_sprinkles::asset::{ParticleSystemAuthors, ParticleSystemDimension};

use crate::assets::example_thumbnail_path;
use crate::io::examples_dir;
use crate::project::{load_project_from_path, OpenProjectEvent};
use crate::ui::tokens::{
    BORDER_COLOR, CORNER_RADIUS_LG, FONT_PATH, TEXT_BODY_COLOR, TEXT_MUTED_COLOR, TEXT_SIZE,
    TEXT_SIZE_SM,
};
use crate::ui::widgets::button::{
    button_base, ButtonClickEvent, ButtonVariant, EditorButton, set_button_variant,
};
use crate::ui::widgets::dialog::{
    CloseDialogEvent, DialogActionEvent, DialogChildrenSlot, EditorDialog, OpenDialogEvent,
};
use crate::ui::widgets::scroll::scrollbar;

pub fn plugin(app: &mut App) {
    app.add_observer(handle_examples_button_click)
        .add_observer(handle_example_card_click)
        .add_observer(handle_open_example)
        .add_systems(
            Update,
            (
                setup_examples_dialog_content,
                cleanup_examples_dialog_state,
            ),
        );
}

#[derive(Component)]
pub struct ExamplesButton;

#[derive(Component)]
struct ExampleCard(String);

#[derive(Component)]
struct ExampleCardActive;

struct ExampleEntry {
    name: String,
    path: String,
    dimension: ParticleSystemDimension,
    thumbnail: String,
    authors: Option<ParticleSystemAuthors>,
}

#[derive(Resource)]
struct ExamplesDialogState {
    entries: Vec<ExampleEntry>,
    active_path: Option<String>,
    dialog_entity: Option<Entity>,
    populated: bool,
}

// TODO: this does synchronous filesystem I/O which could block the main thread with many examples
fn collect_example_entries() -> Vec<ExampleEntry> {
    let examples_dir = examples_dir();
    let Ok(read_dir) = std::fs::read_dir(&examples_dir) else {
        return Vec::new();
    };

    let mut entries: Vec<ExampleEntry> = read_dir
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("ron") {
                return None;
            }
            let asset = load_project_from_path(&path)?;
            let stem = path.file_stem()?.to_string_lossy().to_string();
            Some(ExampleEntry {
                name: asset.name,
                path: format!("examples/{stem}.ron"),
                dimension: asset.dimension,
                thumbnail: example_thumbnail_path(&stem),
                authors: asset.authors,
            })
        })
        .collect();

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    entries
}

fn handle_examples_button_click(
    trigger: On<ButtonClickEvent>,
    buttons: Query<(), With<ExamplesButton>>,
    mut commands: Commands,
) {
    if buttons.get(trigger.entity).is_err() {
        return;
    }

    let entries = collect_example_entries();
    let active_path = entries.first().map(|e| e.path.clone());

    commands.insert_resource(ExamplesDialogState {
        entries,
        active_path,
        dialog_entity: None,
        populated: false,
    });

    commands.trigger(
        OpenDialogEvent::new("Examples", "Open project")
            .without_cancel()
            .without_content_padding()
            .with_max_width(px(600)),
    );
}

fn setup_examples_dialog_content(
    state: Option<ResMut<ExamplesDialogState>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    slots: Query<Entity, With<DialogChildrenSlot>>,
    dialogs: Query<Entity, With<EditorDialog>>,
) {
    let Some(mut state) = state else { return };
    if state.populated {
        return;
    }
    let Ok(slot_entity) = slots.single() else {
        return;
    };

    state.populated = true;
    state.dialog_entity = dialogs.single().ok();

    let font: Handle<Font> = asset_server.load(FONT_PATH);

    let scroll_container = commands
        .spawn((
            Hovered::default(),
            Node {
                width: percent(100),
                max_height: px(384),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();

    commands
        .entity(scroll_container)
        .with_child(scrollbar(scroll_container));

    let grid = commands
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![GridTrack::fr(1.0), GridTrack::fr(1.0)],
            column_gap: px(12),
            row_gap: px(12),
            padding: UiRect::all(px(24)),
            ..default()
        })
        .id();

    for entry in &state.entries {
        let is_active = state
            .active_path
            .as_ref()
            .is_some_and(|p| p == &entry.path);

        let card = spawn_example_card(&mut commands, &asset_server, &font, entry, is_active);
        commands.entity(grid).add_child(card);
    }

    commands.entity(scroll_container).add_child(grid);
    commands.entity(slot_entity).add_child(scroll_container);
}

fn spawn_example_card(
    commands: &mut Commands,
    asset_server: &AssetServer,
    font: &Handle<Font>,
    entry: &ExampleEntry,
    is_active: bool,
) -> Entity {
    let variant = if is_active {
        ButtonVariant::Active
    } else {
        ButtonVariant::Ghost
    };

    let card = commands
        .spawn((
            ExampleCard(entry.path.clone()),
            button_base(variant, Default::default(), false, FlexDirection::Column),
        ))
        .id();

    if is_active {
        commands.entity(card).insert(ExampleCardActive);
    }

    commands
        .entity(card)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.padding = UiRect::all(px(6));
            node.height = Val::Auto;
            node.row_gap = px(3);
            node.align_items = AlignItems::Stretch;
        });

    commands.entity(card).with_child((
        ImageNode::new(asset_server.load(&entry.thumbnail)).with_mode(NodeImageMode::Stretch),
        Node {
            width: percent(100),
            aspect_ratio: Some(16.0 / 9.0),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(CORNER_RADIUS_LG),
            margin: UiRect::bottom(px(3)),
            ..default()
        },
        BorderColor::all(BORDER_COLOR),
    ));

    let dimension_label = match entry.dimension {
        ParticleSystemDimension::D3 => "3D",
        ParticleSystemDimension::D2 => "2D",
    };

    let name_row = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: px(3),
            ..default()
        })
        .with_child((
            Text::new(&entry.name),
            TextFont {
                font: font.clone(),
                font_size: TEXT_SIZE,
                weight: FontWeight::MEDIUM,
                ..default()
            },
            TextColor(TEXT_BODY_COLOR.into()),
        ))
        .with_child((
            Node {
                padding: UiRect::axes(px(3), px(1)),
                border_radius: BorderRadius::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(tailwind::ZINC_500.with_alpha(0.2).into()),
            children![(
                Text::new(dimension_label),
                TextFont {
                    font: font.clone(),
                    font_size: TEXT_SIZE_SM,
                    ..default()
                },
                TextColor(TEXT_BODY_COLOR.into()),
            )],
        ))
        .id();

    commands.entity(card).add_child(name_row);

    if let Some(authors) = &entry.authors {
        let label = match &authors.inspired_by {
            Some(inspired) => format!("Original by: {inspired} · Author: {}", authors.submitted_by),
            None => format!("Author: {}", authors.submitted_by),
        };

        commands.entity(card).with_child((
            Text::new(label),
            TextFont {
                font: font.clone(),
                font_size: TEXT_SIZE_SM,
                ..default()
            },
            TextColor(TEXT_MUTED_COLOR.into()),
        ));
    }

    card
}

fn handle_example_card_click(
    trigger: On<ButtonClickEvent>,
    cards: Query<&ExampleCard>,
    active_cards: Query<Entity, With<ExampleCardActive>>,
    mut variants: Query<(&mut BackgroundColor, &mut BorderColor), With<EditorButton>>,
    state: Option<ResMut<ExamplesDialogState>>,
    mut commands: Commands,
) {
    let Ok(card) = cards.get(trigger.entity) else {
        return;
    };

    let Some(mut state) = state else { return };

    let was_active = state
        .active_path
        .as_ref()
        .is_some_and(|p| p == &card.0);

    if was_active {
        open_active_example(&state, &mut commands);
        return;
    }

    for prev in &active_cards {
        commands.entity(prev).remove::<ExampleCardActive>();
        commands
            .entity(prev)
            .remove::<ButtonVariant>()
            .insert(ButtonVariant::Ghost);
        if let Ok((mut bg, mut border)) = variants.get_mut(prev) {
            set_button_variant(ButtonVariant::Ghost, &mut bg, &mut border);
        }
    }

    commands
        .entity(trigger.entity)
        .insert((ExampleCardActive, ButtonVariant::Active));
    if let Ok((mut bg, mut border)) = variants.get_mut(trigger.entity) {
        set_button_variant(ButtonVariant::Active, &mut bg, &mut border);
    }

    state.active_path = Some(card.0.clone());
}

fn handle_open_example(
    event: On<DialogActionEvent>,
    state: Option<Res<ExamplesDialogState>>,
    mut commands: Commands,
) {
    let Some(state) = state else { return };
    if state.dialog_entity != Some(event.entity) {
        return;
    }
    open_active_example(&state, &mut commands);
}

fn open_active_example(state: &ExamplesDialogState, commands: &mut Commands) {
    let Some(path) = &state.active_path else {
        return;
    };
    commands.trigger(OpenProjectEvent(path.clone()));
    commands.remove_resource::<ExamplesDialogState>();
    commands.trigger(CloseDialogEvent);
}

fn cleanup_examples_dialog_state(
    state: Option<Res<ExamplesDialogState>>,
    dialogs: Query<(), With<EditorDialog>>,
    mut commands: Commands,
) {
    if state.is_some() && dialogs.is_empty() {
        commands.remove_resource::<ExamplesDialogState>();
    }
}
