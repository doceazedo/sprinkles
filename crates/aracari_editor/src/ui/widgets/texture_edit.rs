use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use bevy::color::palettes::tailwind;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::reflect::ReflectRef;
use bevy::tasks::IoTaskPool;

use aracari::prelude::*;
use aracari::textures::preset::{PresetTexture, TextureRef};

use crate::state::EditorState;
use crate::ui::components::binding::{get_inspecting_emitter, resolve_variant_field_ref};
use crate::ui::tokens::{
    BORDER_COLOR, CORNER_RADIUS, FONT_PATH, TEXT_BODY_COLOR, TEXT_MUTED_COLOR, TEXT_SIZE_SM,
};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonSize, ButtonVariant, button, button_base,
    set_button_variant,
};
use crate::ui::widgets::variant_edit::{
    EditorVariantEdit, VariantEditConfig, VariantFieldBinding, VariantFieldsContainer,
};

use crate::ui::components::inspector::FieldKind;
use crate::ui::icons::{ICON_FOLDER_OPEN, ICON_HEART};
use crate::ui::widgets::alert::{AlertSpan, AlertVariant, alert};
use crate::ui::widgets::link::spawn_link_hitbox;

const PRESET_GRID_MAX_HEIGHT: f32 = 256.0;
const PREVIEW_SIZE: f32 = 96.0;

const SCROLLBAR_WIDTH: f32 = 3.0;
const SCROLLBAR_MARGIN: f32 = 3.0;
const SCROLLBAR_MIN_HEIGHT: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextureVariant {
    None,
    Preset,
    Custom,
}

impl TextureVariant {
    fn index(self) -> usize {
        self as usize
    }
}

impl From<usize> for TextureVariant {
    fn from(index: usize) -> Self {
        match index {
            1 => Self::Preset,
            2 => Self::Custom,
            _ => Self::None,
        }
    }
}

#[derive(EntityEvent)]
pub struct TextureEditCommitEvent {
    pub entity: Entity,
    pub value: Option<TextureRef>,
}

#[derive(Component)]
struct TextureEditContent {
    variant_edit: Entity,
    current_variant: TextureVariant,
}

#[derive(Component)]
struct PresetButton {
    variant_edit: Entity,
    preset: PresetTexture,
}

#[derive(Component)]
struct SelectFileButton(Entity);

#[derive(Component)]
struct TexturePreviewImage(Entity);

#[derive(Component)]
struct TexturePathText(Entity);

#[derive(Component)]
struct TextureLocalAlert(Entity);

#[derive(Component)]
struct TextureFileColumn(Entity);

#[derive(Component)]
struct TexturePresetScroll;

#[derive(Component)]
struct TextureGridScrollbar {
    scroll_container: Entity,
}

#[derive(Resource)]
struct TextureFilePickResult {
    variant_edit: Entity,
    result: Arc<Mutex<Option<PathBuf>>>,
}

pub fn plugin(app: &mut App) {
    app.add_observer(handle_preset_click)
        .add_observer(handle_select_file_click)
        .add_systems(
            Update,
            (
                setup_texture_content,
                respawn_texture_content_on_switch,
                update_texture_scrollbar,
                poll_texture_file_pick,
            ),
        );
}

fn is_texture_ref_variant_edit(entity: Entity, bindings: &Query<&VariantFieldBinding>) -> bool {
    bindings
        .get(entity)
        .map(|b| b.field_kind == FieldKind::TextureRef)
        .unwrap_or(false)
}

fn setup_texture_content(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    p_assets: Res<Assets<ParticleSystemAsset>>,
    containers: Query<(Entity, &VariantFieldsContainer), Added<VariantFieldsContainer>>,
    configs: Query<&VariantEditConfig, With<EditorVariantEdit>>,
    bindings: Query<&VariantFieldBinding>,
) {
    for (container_entity, container) in &containers {
        let variant_edit = container.0;

        if !is_texture_ref_variant_edit(variant_edit, &bindings) {
            continue;
        }

        let Ok(config) = configs.get(variant_edit) else {
            continue;
        };

        let current_texture =
            read_current_texture_ref(variant_edit, &editor_state, &p_assets, &bindings, &configs);

        let variant = TextureVariant::from(config.selected_index);

        commands
            .entity(container_entity)
            .insert(TextureEditContent {
                variant_edit,
                current_variant: variant,
            });

        spawn_content_for_variant(
            &mut commands,
            container_entity,
            variant_edit,
            variant,
            current_texture.as_ref(),
            &asset_server,
        );
    }
}

fn respawn_texture_content_on_switch(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    editor_state: Res<EditorState>,
    p_assets: Res<Assets<ParticleSystemAsset>>,
    changed_configs: Query<(Entity, &VariantEditConfig), Changed<VariantEditConfig>>,
    mut containers: Query<(
        Entity,
        &VariantFieldsContainer,
        &mut TextureEditContent,
        Option<&Children>,
    )>,
    bindings: Query<&VariantFieldBinding>,
    configs: Query<&VariantEditConfig, With<EditorVariantEdit>>,
) {
    for (variant_edit, config) in &changed_configs {
        if !is_texture_ref_variant_edit(variant_edit, &bindings) {
            continue;
        }

        for (container_entity, container, mut content, children) in &mut containers {
            if container.0 != variant_edit {
                continue;
            }

            let variant = TextureVariant::from(config.selected_index);

            if content.current_variant == variant {
                continue;
            }

            if let Some(children) = children {
                for child in children.iter() {
                    commands.entity(child).try_despawn();
                }
            }

            content.current_variant = variant;

            let current_texture = read_current_texture_ref(
                variant_edit,
                &editor_state,
                &p_assets,
                &bindings,
                &configs,
            );

            spawn_content_for_variant(
                &mut commands,
                container_entity,
                variant_edit,
                variant,
                current_texture.as_ref(),
                &asset_server,
            );

            break;
        }
    }
}

fn spawn_content_for_variant(
    commands: &mut Commands,
    container: Entity,
    variant_edit: Entity,
    variant: TextureVariant,
    current_texture: Option<&TextureRef>,
    asset_server: &AssetServer,
) {
    match variant {
        TextureVariant::None => {}
        TextureVariant::Preset => {
            let current_preset = current_texture.and_then(|t| match t {
                TextureRef::Preset(p) => Some(p),
                _ => None,
            });
            spawn_preset_grid(
                commands,
                container,
                variant_edit,
                current_preset,
                asset_server,
            );
        }
        TextureVariant::Custom => {
            spawn_file_content(
                commands,
                container,
                variant_edit,
                current_texture,
                asset_server,
            );
        }
    }
}

fn spawn_preset_grid(
    commands: &mut Commands,
    container: Entity,
    variant_edit: Entity,
    current_preset: Option<&PresetTexture>,
    asset_server: &AssetServer,
) {
    let scroll_container = commands
        .spawn((
            TexturePresetScroll,
            Hovered::default(),
            Node {
                max_height: px(PRESET_GRID_MAX_HEIGHT),
                overflow: Overflow::scroll_y(),
                width: percent(100),
                position_type: PositionType::Relative,
                ..default()
            },
        ))
        .id();

    let grid = commands
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::flex(4, 1.0)],
            column_gap: px(4.0),
            row_gap: px(4.0),
            width: percent(100),
            ..default()
        })
        .id();

    for preset in PresetTexture::all() {
        let is_active = current_preset == Some(&preset);
        let variant = if is_active {
            ButtonVariant::Active
        } else {
            ButtonVariant::Ghost
        };

        let btn = commands
            .spawn((
                PresetButton {
                    variant_edit,
                    preset: preset.clone(),
                },
                button_base(variant, ButtonSize::MD, false, FlexDirection::Row),
            ))
            .insert(Node {
                aspect_ratio: Some(1.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(4.0)),
                overflow: Overflow::clip(),
                ..default()
            })
            .with_child((
                ImageNode::new(asset_server.load(preset.embedded_path())),
                Node {
                    width: percent(100),
                    height: percent(100),
                    border_radius: BorderRadius::all(px(4.0)),
                    ..default()
                },
            ))
            .id();

        commands.entity(grid).add_child(btn);
    }

    let scrollbar = commands
        .spawn((
            TextureGridScrollbar { scroll_container },
            Node {
                position_type: PositionType::Absolute,
                width: px(SCROLLBAR_WIDTH),
                right: px(SCROLLBAR_MARGIN),
                top: px(SCROLLBAR_MARGIN),
                border_radius: BorderRadius::all(px(SCROLLBAR_WIDTH / 2.0)),
                ..default()
            },
            IgnoreScroll(BVec2::new(false, true)),
            BackgroundColor(tailwind::ZINC_600.into()),
            Visibility::Hidden,
        ))
        .id();

    commands.entity(scroll_container).add_child(grid);
    commands.entity(scroll_container).add_child(scrollbar);
    commands.entity(container).add_child(scroll_container);

    spawn_footnote(commands, container, asset_server);
}

fn spawn_footnote(commands: &mut Commands, parent: Entity, asset_server: &AssetServer) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);
    let text_color: Color = TEXT_MUTED_COLOR.into();
    let link_color: Color = TEXT_BODY_COLOR.into();

    let row = commands
        .spawn(Node {
            align_items: AlignItems::Center,
            column_gap: px(3),
            ..default()
        })
        .id();

    let icon = commands
        .spawn((
            ImageNode::new(asset_server.load(ICON_HEART)).with_color(tailwind::PINK_600.into()),
            Node {
                width: px(14),
                height: px(14),
                ..default()
            },
        ))
        .id();
    commands.entity(row).add_child(icon);

    let text_id = commands
        .spawn((
            Text::new("Textures by "),
            TextFont {
                font: font.clone(),
                font_size: TEXT_SIZE_SM,
                ..default()
            },
            TextColor(text_color),
        ))
        .id();

    let link_span = commands
        .spawn((
            TextSpan::new("Kenney"),
            TextFont {
                font: font.clone(),
                font_size: TEXT_SIZE_SM,
                weight: FontWeight::MEDIUM,
                ..default()
            },
            TextColor(link_color),
            Underline,
        ))
        .id();
    commands.entity(text_id).add_child(link_span);

    let suffix = commands
        .spawn((
            TextSpan::new(" under CC0 license."),
            TextFont {
                font,
                font_size: TEXT_SIZE_SM,
                ..default()
            },
            TextColor(text_color),
        ))
        .id();
    commands.entity(text_id).add_child(suffix);

    let text_wrapper = commands
        .spawn(Node {
            position_type: PositionType::Relative,
            ..default()
        })
        .id();

    let hitbox = spawn_link_hitbox(
        commands,
        text_id,
        1,
        link_span,
        "https://kenney.nl".to_string(),
        link_color,
    );

    commands.entity(text_wrapper).add_child(text_id);
    commands.entity(text_wrapper).add_child(hitbox);
    commands.entity(row).add_child(text_wrapper);
    commands.entity(parent).add_child(row);
}

fn spawn_file_content(
    commands: &mut Commands,
    container: Entity,
    variant_edit: Entity,
    current_texture: Option<&TextureRef>,
    asset_server: &AssetServer,
) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    let texture_path = current_texture.and_then(|t| match t {
        TextureRef::Asset(p) | TextureRef::Local(p) if !p.is_empty() => Some(p.as_str()),
        _ => None,
    });

    let column = commands
        .spawn((
            TextureFileColumn(variant_edit),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                align_items: AlignItems::Center,
                width: percent(100),
                ..default()
            },
        ))
        .id();

    let preview_wrapper = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(6.0),
            ..default()
        })
        .id();

    let preview = commands
        .spawn((
            TexturePreviewImage(variant_edit),
            Node {
                width: px(PREVIEW_SIZE),
                height: px(PREVIEW_SIZE),
                border_radius: BorderRadius::all(CORNER_RADIUS),
                border: UiRect::all(px(1.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BorderColor::all(BORDER_COLOR),
        ))
        .id();
    if let Some(path) = texture_path {
        let image = commands
            .spawn((
                ImageNode::new(asset_server.load(path.to_owned())),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
            ))
            .id();
        commands.entity(preview).add_child(image);
    }
    commands.entity(preview_wrapper).add_child(preview);

    let display_path = texture_path
        .map(|p| format_display_path(p))
        .unwrap_or_else(|| "No file selected".to_string());
    let path_text = commands
        .spawn((
            TexturePathText(variant_edit),
            Text::new(&display_path),
            TextFont {
                font,
                font_size: TEXT_SIZE_SM,
                ..default()
            },
            TextColor(TEXT_MUTED_COLOR.into()),
        ))
        .id();
    commands.entity(preview_wrapper).add_child(path_text);

    commands.entity(column).add_child(preview_wrapper);

    let btn = commands
        .spawn((
            SelectFileButton(variant_edit),
            button(ButtonProps::new("Select file...").with_left_icon(ICON_FOLDER_OPEN)),
        ))
        .id();
    commands.entity(column).add_child(btn);

    if matches!(current_texture, Some(TextureRef::Local(_))) {
        spawn_local_texture_alert(commands, column, variant_edit);
    }

    commands.entity(container).add_child(column);
}

fn spawn_local_texture_alert(commands: &mut Commands, parent: Entity, variant_edit: Entity) {
    let alert_entity = commands
        .spawn((
            TextureLocalAlert(variant_edit),
            alert(
                AlertVariant::Important,
                vec![
                    AlertSpan::Text("This texture is outside your game's ".into()),
                    AlertSpan::Bold("\"assets\"".into()),
                    AlertSpan::Text(" folder, and might not load in the actual game. ".into()),
                    AlertSpan::Link {
                        text: "Learn more.".into(),
                        // TODO: link to TextureRef::Local docs
                        url: "https://github.com/doceazedo/aracari/issues/new?title=hey+dumbass&body=you+made+the+repo+public+and+forgot+to+add+the+link+to+the+TextureRef%3A%3ALocal+docs".into(),
                    },
                ],
            ),
        ))
        .id();
    commands.entity(parent).add_child(alert_entity);
}

fn update_texture_scrollbar(
    scroll_containers: Query<(&Hovered, &ScrollPosition, &ComputedNode), With<TexturePresetScroll>>,
    mut scrollbars: Query<(&TextureGridScrollbar, &mut Node, &mut Visibility)>,
) {
    for (scrollbar, mut node, mut visibility) in &mut scrollbars {
        let Ok((hovered, scroll_position, computed)) =
            scroll_containers.get(scrollbar.scroll_container)
        else {
            continue;
        };

        let content_height = computed.content_size().y * computed.inverse_scale_factor();
        let visible_height = computed.size().y * computed.inverse_scale_factor();
        let has_scroll = content_height > visible_height;

        let should_show = hovered.get() && has_scroll;
        let new_visibility = if should_show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        if *visibility != new_visibility {
            *visibility = new_visibility;
        }

        if !has_scroll {
            continue;
        }

        let track_height = visible_height - (SCROLLBAR_MARGIN * 2.0);
        let thumb_ratio = visible_height / content_height;
        let thumb_height = (track_height * thumb_ratio).max(SCROLLBAR_MIN_HEIGHT);

        let max_scroll = content_height - visible_height;
        let scroll_ratio = if max_scroll > 0.0 {
            scroll_position.y / max_scroll
        } else {
            0.0
        };
        let thumb_offset = scroll_ratio * (track_height - thumb_height);

        node.top = px(SCROLLBAR_MARGIN + thumb_offset);
        node.height = px(thumb_height);
    }
}

fn handle_preset_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    preset_buttons: Query<(Entity, &PresetButton)>,
    mut configs: Query<&mut VariantEditConfig, With<EditorVariantEdit>>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
) {
    let Ok((_, preset_btn)) = preset_buttons.get(trigger.entity) else {
        return;
    };

    let variant_edit = preset_btn.variant_edit;
    let clicked_preset = preset_btn.preset.clone();
    let value = Some(TextureRef::Preset(clicked_preset.clone()));

    for (entity, btn) in &preset_buttons {
        if btn.variant_edit != variant_edit {
            continue;
        }
        if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(entity) {
            if btn.preset == clicked_preset {
                *variant = ButtonVariant::Active;
                set_button_variant(ButtonVariant::Active, &mut bg, &mut border);
            } else {
                *variant = ButtonVariant::Ghost;
                set_button_variant(ButtonVariant::Ghost, &mut bg, &mut border);
            }
        }
    }

    commands.trigger(TextureEditCommitEvent {
        entity: variant_edit,
        value,
    });

    if let Ok(mut config) = configs.get_mut(variant_edit) {
        config.selected_index = TextureVariant::Preset.index();
    }
}

fn handle_select_file_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    select_buttons: Query<&SelectFileButton>,
) {
    let Ok(select_btn) = select_buttons.get(trigger.entity) else {
        return;
    };

    let variant_edit = select_btn.0;
    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();

    let task = rfd::AsyncFileDialog::new()
        .set_title("Select Texture")
        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "tga", "webp"])
        .pick_file();

    IoTaskPool::get()
        .spawn(async move {
            if let Some(file_handle) = task.await {
                let path = file_handle.path().to_path_buf();
                if let Ok(mut guard) = result_clone.lock() {
                    *guard = Some(path);
                }
            }
        })
        .detach();

    commands.insert_resource(TextureFilePickResult {
        variant_edit,
        result,
    });
}

fn poll_texture_file_pick(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pick_result: Option<Res<TextureFilePickResult>>,
    mut configs: Query<&mut VariantEditConfig, With<EditorVariantEdit>>,
    preview_images: Query<(Entity, &TexturePreviewImage, Option<&Children>)>,
    mut path_texts: Query<(&TexturePathText, &mut Text)>,
    existing_alerts: Query<(Entity, &TextureLocalAlert)>,
    columns: Query<(Entity, &TextureFileColumn)>,
) {
    let Some(pick) = pick_result else {
        return;
    };

    let path = {
        let Ok(guard) = pick.result.try_lock() else {
            return;
        };
        guard.clone()
    };

    let Some(path) = path else {
        return;
    };

    let variant_edit = pick.variant_edit;
    commands.remove_resource::<TextureFilePickResult>();

    let path_str = path.to_string_lossy().to_string();
    let texture_ref = classify_texture_path(&path_str);

    let load_path = match &texture_ref {
        TextureRef::Asset(p) => p.clone(),
        TextureRef::Local(p) => p.clone(),
        _ => String::new(),
    };

    for (entity, preview, children) in &preview_images {
        if preview.0 != variant_edit {
            continue;
        }
        if let Some(children) = children {
            for child in children.iter() {
                commands.entity(child).try_despawn();
            }
        }
        if !load_path.is_empty() {
            let image = commands
                .spawn((
                    ImageNode::new(asset_server.load(load_path.clone())),
                    Node {
                        width: percent(100),
                        height: percent(100),
                        ..default()
                    },
                ))
                .id();
            commands.entity(entity).add_child(image);
        }
    }

    let display_path = match &texture_ref {
        TextureRef::Asset(p) | TextureRef::Local(p) => format_display_path(p),
        _ => "No file selected".to_string(),
    };
    for (path_text, mut text) in &mut path_texts {
        if path_text.0 == variant_edit {
            **text = display_path.clone();
        }
    }

    let is_local = matches!(texture_ref, TextureRef::Local(_));

    for (alert_entity, alert_marker) in &existing_alerts {
        if alert_marker.0 == variant_edit {
            commands.entity(alert_entity).try_despawn();
        }
    }

    if is_local {
        if let Some((column_entity, _)) = columns.iter().find(|(_, c)| c.0 == variant_edit) {
            spawn_local_texture_alert(&mut commands, column_entity, variant_edit);
        }
    }

    commands.trigger(TextureEditCommitEvent {
        entity: variant_edit,
        value: Some(texture_ref),
    });

    if let Ok(mut config) = configs.get_mut(variant_edit) {
        config.selected_index = TextureVariant::Custom.index();
    }
}

fn read_current_texture_ref(
    variant_edit: Entity,
    editor_state: &EditorState,
    assets: &Assets<ParticleSystemAsset>,
    bindings: &Query<&VariantFieldBinding>,
    configs: &Query<&VariantEditConfig, With<EditorVariantEdit>>,
) -> Option<TextureRef> {
    let binding = bindings.get(variant_edit).ok()?;
    let parent_config = configs.get(binding.variant_edit).ok()?;
    let (_, emitter) = get_inspecting_emitter(editor_state, assets)?;
    let path = format!(".{}", parent_config.path);
    let target = emitter.reflect_path(path.as_str()).ok()?;
    let field = resolve_variant_field_ref(target, &binding.field_name)?;
    extract_texture_ref_from_reflect(field)
}

fn extract_texture_ref_from_reflect(value: &dyn PartialReflect) -> Option<TextureRef> {
    let ReflectRef::Enum(option_enum) = value.reflect_ref() else {
        return None;
    };
    if option_enum.variant_name() != "Some" {
        return None;
    }
    let inner = option_enum.field_at(0)?;
    let ReflectRef::Enum(texture_ref_enum) = inner.reflect_ref() else {
        return None;
    };
    match texture_ref_enum.variant_name() {
        "Preset" => {
            let field = texture_ref_enum.field_at(0)?;
            let preset = field.try_downcast_ref::<PresetTexture>()?.clone();
            Some(TextureRef::Preset(preset))
        }
        "Asset" => {
            let field = texture_ref_enum.field_at(0)?;
            let path = field.try_downcast_ref::<String>()?.clone();
            Some(TextureRef::Asset(path))
        }
        "Local" => {
            let field = texture_ref_enum.field_at(0)?;
            let path = field.try_downcast_ref::<String>()?.clone();
            Some(TextureRef::Local(path))
        }
        _ => None,
    }
}

const MAX_DISPLAY_PATH_LEN: usize = 48;

fn format_display_path(path: &str) -> String {
    let simplified = simplify_path(Path::new(path));
    truncate_middle(&simplified, MAX_DISPLAY_PATH_LEN)
}

fn simplify_path(path: &Path) -> String {
    let cwd = crate::io::working_dir();
    if let Ok(relative) = path.strip_prefix(&cwd) {
        return format!("./{}", relative.to_string_lossy());
    }

    #[cfg(unix)]
    if let Some(home) = std::env::var_os("HOME") {
        let home_path = PathBuf::from(home);
        if let Ok(relative) = path.strip_prefix(&home_path) {
            return format!("~/{}", relative.to_string_lossy());
        }
    }

    path.to_string_lossy().to_string()
}

fn truncate_middle(s: &str, max_len: usize) -> String {
    if s.len() < max_len {
        return s.to_string();
    }
    let keep = max_len.saturating_sub(3);
    let head = keep / 2;
    let tail = keep - head;
    format!("{}...{}", &s[..head], &s[s.len() - tail..])
}

// TODO: `/data/assets/src-backup/` would be wrongly classified
fn classify_texture_path(path: &str) -> TextureRef {
    if let Some(assets_pos) = path.find("/assets/") {
        let before = &path[..assets_pos];
        if !before.contains("/src/") {
            let relative = &path[assets_pos + "/assets/".len()..];
            return TextureRef::Asset(relative.to_string());
        }
    }
    TextureRef::Local(path.to_string())
}
