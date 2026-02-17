use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_easings::{CustomComponentEase, EaseFunction, EasingComponent, EasingType, Lerp};

use bevy_ui_text_input::TextInputPrompt;

use crate::ui::icons::ICON_CLOSE;
use crate::ui::tokens::{
    BACKGROUND_COLOR, BORDER_COLOR, FONT_PATH, TEXT_DISPLAY_COLOR, TEXT_MUTED_COLOR, TEXT_SIZE_LG,
    TEXT_SIZE_XL,
};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonVariant, EditorButton, IconButtonProps, button,
    icon_button,
};

const ANIMATION_DURATION: Duration = Duration::from_millis(200);
const DIALOG_ANIMATION_OFFSET: f32 = 12.0;
const BACKDROP_TARGET_OPACITY: f32 = 0.8;

pub fn plugin(app: &mut App) {
    app.add_observer(on_open_dialog)
        .add_observer(on_open_confirmation_dialog)
        .add_observer(on_action_button_click)
        .add_observer(on_cancel_button_click)
        .add_observer(on_close_button_click)
        .add_observer(on_close_dialog)
        .add_systems(
            Update,
            (
                bevy_easings::custom_ease_system::<(), DialogVisual>,
                sync_dialog_visual,
                sync_children_slot_visibility,
                handle_backdrop_click,
                handle_esc_key,
                handle_dialog_despawn,
            ),
        );
}

#[derive(Component)]
pub struct EditorDialog;

#[derive(Component)]
struct DialogBackdrop;

#[derive(Component)]
struct DialogPanel;

#[derive(Component)]
struct DialogCloseButton;

#[derive(Component)]
struct DialogCancelButton;

#[derive(Component)]
struct DialogActionButton;

#[derive(Component)]
pub struct DialogChildrenSlot;

#[derive(Component, Default, Clone, Copy)]
pub enum DialogVariant {
    #[default]
    Default,
    Destructive,
}

impl DialogVariant {
    fn action_button_variant(&self) -> ButtonVariant {
        match self {
            Self::Default => ButtonVariant::Primary,
            Self::Destructive => ButtonVariant::Destructive,
        }
    }
}

#[derive(Component)]
struct DialogConfig {
    close_on_click_outside: bool,
    close_on_esc: bool,
}

#[derive(EntityEvent)]
pub struct DialogActionEvent {
    pub entity: Entity,
}

#[derive(Event)]
pub struct CloseDialogEvent;

#[derive(Event)]
pub struct OpenDialogEvent {
    pub title: Option<String>,
    pub description: Option<String>,
    pub action: Option<String>,
    pub cancel: Option<String>,
    pub variant: DialogVariant,
    pub has_close_button: bool,
    pub close_on_click_outside: bool,
    pub close_on_esc: bool,
    pub max_width: Option<Val>,
    pub content_padding: UiRect,
}

impl OpenDialogEvent {
    pub fn new(title: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            title: Some(title.into()),
            description: None,
            action: Some(action.into()),
            cancel: Some("Cancel".into()),
            variant: DialogVariant::Default,
            has_close_button: true,
            close_on_click_outside: true,
            close_on_esc: true,
            max_width: None,
            content_padding: UiRect::all(px(24)),
        }
    }

    pub fn without_cancel(mut self) -> Self {
        self.cancel = None;
        self
    }

    pub fn with_variant(mut self, variant: DialogVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn with_close_button(mut self, has_close_button: bool) -> Self {
        self.has_close_button = has_close_button;
        self
    }

    pub fn with_close_on_click_outside(mut self, close_on_click_outside: bool) -> Self {
        self.close_on_click_outside = close_on_click_outside;
        self
    }

    pub fn with_max_width(mut self, max_width: Val) -> Self {
        self.max_width = Some(max_width);
        self
    }

    pub fn without_content_padding(mut self) -> Self {
        self.content_padding = UiRect::ZERO;
        self
    }
}

#[derive(Event)]
pub struct OpenConfirmationDialogEvent {
    pub title: String,
    pub description: Option<String>,
    pub action: String,
}

impl OpenConfirmationDialogEvent {
    pub fn new(title: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            action: action.into(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl From<&OpenConfirmationDialogEvent> for OpenDialogEvent {
    fn from(event: &OpenConfirmationDialogEvent) -> Self {
        let mut dialog = OpenDialogEvent::new(&event.title, &event.action)
            .with_variant(DialogVariant::Destructive)
            .with_close_button(false)
            .with_close_on_click_outside(false);
        dialog.description = event.description.clone();
        dialog
    }
}

#[derive(Component, Clone)]
struct DialogVisual {
    scale: Vec2,
    opacity: f32,
    offset_y: f32,
}

impl Default for DialogVisual {
    fn default() -> Self {
        Self {
            scale: Vec2::ONE,
            opacity: 1.0,
            offset_y: 0.0,
        }
    }
}

impl Lerp for DialogVisual {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            scale: self.scale.lerp(other.scale, *scalar),
            opacity: self.opacity.lerp(other.opacity, *scalar),
            offset_y: self.offset_y.lerp(other.offset_y, *scalar),
        }
    }
}

#[derive(Component)]
struct PromptBaseAlpha(f32);

#[derive(Component)]
struct BaseBgAlpha(f32);

#[derive(Component)]
struct DespawningDialog;

fn on_open_dialog(
    event: On<OpenDialogEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing: Query<Entity, With<EditorDialog>>,
) {
    if !existing.is_empty() {
        return;
    }

    spawn_dialog(&mut commands, &asset_server, &event);
}

fn on_open_confirmation_dialog(
    event: On<OpenConfirmationDialogEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    existing: Query<Entity, With<EditorDialog>>,
) {
    if !existing.is_empty() {
        return;
    }

    let dialog_event: OpenDialogEvent = event.event().into();
    spawn_dialog(&mut commands, &asset_server, &dialog_event);
}

fn spawn_dialog(commands: &mut Commands, asset_server: &AssetServer, event: &OpenDialogEvent) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    let start_visual = DialogVisual {
        scale: Vec2::splat(0.9),
        opacity: 0.0,
        offset_y: DIALOG_ANIMATION_OFFSET,
    };
    let end_visual = DialogVisual {
        scale: Vec2::ONE,
        opacity: 1.0,
        offset_y: 0.0,
    };

    let backdrop_id = commands
        .spawn((
            DialogBackdrop,
            Interaction::None,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.0)),
        ))
        .id();

    let has_header = event.title.is_some() || event.description.is_some();
    let has_footer = event.action.is_some() || event.cancel.is_some();

    let header_id = if has_header {
        let mut header = commands.spawn((
            Node {
                padding: UiRect::all(px(24)),
                border: UiRect::bottom(px(1)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                ..default()
            },
            BorderColor::all(BORDER_COLOR.with_alpha(0.0)),
        ));

        if let Some(title) = &event.title {
            header.with_child((
                Text::new(title),
                TextFont {
                    font: font.clone(),
                    font_size: TEXT_SIZE_XL,
                    weight: FontWeight::SEMIBOLD,
                    ..default()
                },
                TextColor(TEXT_DISPLAY_COLOR.with_alpha(0.0).into()),
            ));
        }

        if let Some(desc) = &event.description {
            header.with_child((
                Text::new(desc),
                TextFont {
                    font: font.clone(),
                    font_size: TEXT_SIZE_LG,
                    ..default()
                },
                TextColor(TEXT_MUTED_COLOR.with_alpha(0.0).into()),
            ));
        }

        Some(header.id())
    } else {
        None
    };

    let footer_id = if has_footer {
        let mut footer = commands.spawn(Node {
            padding: UiRect::all(px(24)),
            column_gap: px(6),
            justify_content: JustifyContent::End,
            ..default()
        });

        if let Some(cancel) = &event.cancel {
            footer.with_child((DialogCancelButton, button(ButtonProps::new(cancel))));
        }

        if let Some(action) = &event.action {
            footer.with_child((
                DialogActionButton,
                button(
                    ButtonProps::new(action).with_variant(event.variant.action_button_variant()),
                ),
            ));
        }

        Some(footer.id())
    } else {
        None
    };

    let mut panel = commands.spawn((
        DialogPanel,
        Interaction::None,
        Node {
            width: percent(100),
            max_width: event.max_width.unwrap_or(px(448)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(BACKGROUND_COLOR.with_alpha(0.0).into()),
        BorderColor::all(BORDER_COLOR.with_alpha(0.0)),
        UiTransform {
            scale: Vec2::splat(0.9),
            ..default()
        },
    ));

    if let Some(header_id) = header_id {
        panel.add_child(header_id);
    }

    panel.with_child((
        DialogChildrenSlot,
        Node {
            display: Display::None,
            padding: event.content_padding,
            border: UiRect::bottom(px(1)),
            flex_direction: FlexDirection::Column,
            row_gap: px(12),
            ..default()
        },
        BorderColor::all(BORDER_COLOR.with_alpha(0.0)),
    ));

    if let Some(footer_id) = footer_id {
        panel.add_child(footer_id);
    }

    if event.has_close_button {
        panel.with_child((
            Node {
                position_type: PositionType::Absolute,
                top: px(20),
                right: px(20),
                ..default()
            },
            children![(
                DialogCloseButton,
                icon_button(
                    IconButtonProps::new(ICON_CLOSE).variant(ButtonVariant::Ghost),
                    asset_server,
                ),
            )],
        ));
    }

    let panel_id = panel.id();

    commands.entity(backdrop_id).add_child(panel_id);

    commands
        .spawn((
            EditorDialog,
            event.variant,
            DialogConfig {
                close_on_click_outside: event.close_on_click_outside,
                close_on_esc: event.close_on_esc,
            },
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                ..default()
            },
            GlobalZIndex(200),
            Pickable::IGNORE,
            start_visual
                .ease_to(
                    end_visual,
                    EaseFunction::QuinticOut,
                    EasingType::Once {
                        duration: ANIMATION_DURATION,
                    },
                )
                .with_original_value(),
        ))
        .add_child(backdrop_id);
}

fn dismiss_dialog(commands: &mut Commands, entity: Entity, visual: &DialogVisual) {
    let end_visual = DialogVisual {
        scale: Vec2::splat(0.9),
        opacity: 0.0,
        offset_y: visual.offset_y + DIALOG_ANIMATION_OFFSET,
    };

    commands.entity(entity).insert((
        DespawningDialog,
        visual
            .clone()
            .ease_to(
                end_visual,
                EaseFunction::QuinticOut,
                EasingType::Once {
                    duration: ANIMATION_DURATION,
                },
            )
            .with_original_value(),
    ));
}

#[derive(SystemParam)]
struct AlphaQueries<'w, 's> {
    transforms: Query<'w, 's, &'static mut UiTransform>,
    bg_colors: Query<'w, 's, &'static mut BackgroundColor>,
    border_colors: Query<'w, 's, &'static mut BorderColor>,
    text_colors: Query<'w, 's, &'static mut TextColor>,
    image_nodes: Query<'w, 's, &'static mut ImageNode>,
    prompts: Query<'w, 's, (Entity, &'static mut TextInputPrompt)>,
    base_alphas: Query<'w, 's, &'static PromptBaseAlpha>,
    base_bg_alphas: Query<'w, 's, &'static BaseBgAlpha>,
    children: Query<'w, 's, &'static Children>,
    buttons: Query<'w, 's, &'static ButtonVariant, With<EditorButton>>,
}

impl AlphaQueries<'_, '_> {
    fn apply_recursive(
        &mut self,
        entity: Entity,
        alpha: f32,
        pending_base_bg: &mut Vec<(Entity, f32)>,
    ) {
        if let Ok(variant) = self.buttons.get(entity) {
            if let Ok(mut bg) = self.bg_colors.get_mut(entity) {
                bg.0 = variant
                    .bg_color(false)
                    .with_alpha(variant.bg_opacity(false) * alpha)
                    .into();
            }
            if let Ok(mut border) = self.border_colors.get_mut(entity) {
                *border = BorderColor::all(
                    variant
                        .border_color()
                        .with_alpha(variant.border_opacity(false) * alpha),
                );
            }
        } else {
            if let Ok(mut bg) = self.bg_colors.get_mut(entity) {
                let base: Srgba = bg.0.into();
                let base_alpha = if let Ok(stored) = self.base_bg_alphas.get(entity) {
                    stored.0
                } else {
                    pending_base_bg.push((entity, base.alpha));
                    base.alpha
                };
                bg.0 = base.with_alpha(base_alpha * alpha).into();
            }
            if let Ok(mut border) = self.border_colors.get_mut(entity) {
                let base: Srgba = border.top.into();
                *border = BorderColor::all(base.with_alpha(alpha));
            }
        }

        if let Ok(mut text_color) = self.text_colors.get_mut(entity) {
            let base: Srgba = text_color.0.into();
            text_color.0 = base.with_alpha(alpha).into();
        }

        if let Ok(mut image) = self.image_nodes.get_mut(entity) {
            let base: Srgba = image.color.into();
            image.color = base.with_alpha(alpha).into();
        }

        if let Ok((_, mut prompt)) = self.prompts.get_mut(entity) {
            if let Some(color) = &mut prompt.color {
                let base: Srgba = (*color).into();
                let base_alpha = self
                    .base_alphas
                    .get(entity)
                    .map(|b| b.0)
                    .unwrap_or(base.alpha);
                *color = base.with_alpha(base_alpha * alpha).into();
            }
        }

        if let Ok(children) = self.children.get(entity) {
            let children: Vec<Entity> = children.iter().collect();
            for child in children {
                self.apply_recursive(child, alpha, pending_base_bg);
            }
        }
    }

    fn sync_panel(
        &mut self,
        panel: Entity,
        visual: &DialogVisual,
        alpha: f32,
        pending_base_bg: &mut Vec<(Entity, f32)>,
    ) {
        if let Ok(mut transform) = self.transforms.get_mut(panel) {
            transform.scale = visual.scale;
            transform.translation.y = px(visual.offset_y);
        }

        if let Ok(mut bg) = self.bg_colors.get_mut(panel) {
            bg.0 = BACKGROUND_COLOR.with_alpha(alpha).into();
        }
        if let Ok(mut border) = self.border_colors.get_mut(panel) {
            *border = BorderColor::all(BORDER_COLOR.with_alpha(alpha));
        }

        if let Ok(children) = self.children.get(panel) {
            let children: Vec<Entity> = children.iter().collect();
            for child in children {
                self.apply_recursive(child, alpha, pending_base_bg);
            }
        }
    }
}

fn sync_dialog_visual(
    dialogs: Query<(&DialogVisual, &Children), Changed<DialogVisual>>,
    mut commands: Commands,
    mut alpha_queries: AlphaQueries,
    backdrop_query: Query<Entity, With<DialogBackdrop>>,
) {
    if !dialogs.is_empty() {
        for (entity, prompt) in &alpha_queries.prompts {
            if alpha_queries.base_alphas.contains(entity) {
                continue;
            }
            if let Some(color) = prompt.color {
                let base: Srgba = color.into();
                commands.entity(entity).insert(PromptBaseAlpha(base.alpha));
            }
        }
    }

    let mut pending_base_bg = Vec::new();

    for (visual, dialog_children) in &dialogs {
        let alpha = visual.opacity;

        for child in dialog_children.iter() {
            if !backdrop_query.contains(child) {
                continue;
            }

            if let Ok(mut bg) = alpha_queries.bg_colors.get_mut(child) {
                bg.0 = Color::BLACK.with_alpha(BACKDROP_TARGET_OPACITY * alpha);
            }

            let Ok(backdrop_children) = alpha_queries.children.get(child) else {
                continue;
            };
            let panels: Vec<Entity> = backdrop_children.iter().collect();

            for panel in panels {
                alpha_queries.sync_panel(panel, visual, alpha, &mut pending_base_bg);
            }
        }
    }

    for (entity, alpha) in pending_base_bg {
        commands.entity(entity).insert(BaseBgAlpha(alpha));
    }
}

fn sync_children_slot_visibility(
    mut slots: Query<(&Children, &mut Node), (With<DialogChildrenSlot>, Changed<Children>)>,
) {
    for (children, mut node) in &mut slots {
        node.display = if children.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
    }
}

fn handle_backdrop_click(
    interactions: Query<(&Interaction, &ChildOf), (Changed<Interaction>, With<DialogBackdrop>)>,
    panels: Query<&Interaction, With<DialogPanel>>,
    dialogs: Query<(&DialogConfig, &DialogVisual), (With<EditorDialog>, Without<DespawningDialog>)>,
    mut commands: Commands,
) {
    for (interaction, child_of) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let Ok((config, visual)) = dialogs.get(child_of.parent()) else {
            continue;
        };

        if !config.close_on_click_outside {
            continue;
        }

        if panels.iter().any(|i| *i == Interaction::Pressed) {
            continue;
        }

        dismiss_dialog(&mut commands, child_of.parent(), visual);
    }
}

fn handle_esc_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    dialogs: Query<
        (Entity, &DialogConfig, &DialogVisual),
        (With<EditorDialog>, Without<DespawningDialog>),
    >,
    mut commands: Commands,
) {
    if !keyboard.just_pressed(KeyCode::Escape) {
        return;
    }

    for (entity, config, visual) in &dialogs {
        if config.close_on_esc {
            dismiss_dialog(&mut commands, entity, visual);
        }
    }
}

fn on_close_dialog(
    _event: On<CloseDialogEvent>,
    dialogs: Query<(Entity, &DialogVisual), (With<EditorDialog>, Without<DespawningDialog>)>,
    mut commands: Commands,
) {
    for (entity, visual) in &dialogs {
        dismiss_dialog(&mut commands, entity, visual);
    }
}

fn on_action_button_click(
    event: On<ButtonClickEvent>,
    action_buttons: Query<&ChildOf, With<DialogActionButton>>,
    parents: Query<&ChildOf>,
    dialogs: Query<(Entity, &DialogVisual), (With<EditorDialog>, Without<DespawningDialog>)>,
    mut commands: Commands,
) {
    let Ok(button_parent) = action_buttons.get(event.entity) else {
        return;
    };

    if let Some(dialog_entity) =
        find_and_dismiss(button_parent.parent(), &parents, &dialogs, &mut commands)
    {
        commands.trigger(DialogActionEvent {
            entity: dialog_entity,
        });
    }
}

fn on_cancel_button_click(
    event: On<ButtonClickEvent>,
    cancel_buttons: Query<&ChildOf, With<DialogCancelButton>>,
    parents: Query<&ChildOf>,
    dialogs: Query<(Entity, &DialogVisual), (With<EditorDialog>, Without<DespawningDialog>)>,
    mut commands: Commands,
) {
    let Ok(button_parent) = cancel_buttons.get(event.entity) else {
        return;
    };

    find_and_dismiss(button_parent.parent(), &parents, &dialogs, &mut commands);
}

fn on_close_button_click(
    event: On<ButtonClickEvent>,
    close_buttons: Query<&ChildOf, With<DialogCloseButton>>,
    parents: Query<&ChildOf>,
    dialogs: Query<(Entity, &DialogVisual), (With<EditorDialog>, Without<DespawningDialog>)>,
    mut commands: Commands,
) {
    let Ok(button_parent) = close_buttons.get(event.entity) else {
        return;
    };

    find_and_dismiss(button_parent.parent(), &parents, &dialogs, &mut commands);
}

fn find_and_dismiss(
    start: Entity,
    parents: &Query<&ChildOf>,
    dialogs: &Query<(Entity, &DialogVisual), (With<EditorDialog>, Without<DespawningDialog>)>,
    commands: &mut Commands,
) -> Option<Entity> {
    let (dialog_entity, visual) = find_dialog_ancestor(start, parents, dialogs)?;
    dismiss_dialog(commands, dialog_entity, visual);
    Some(dialog_entity)
}

fn find_dialog_ancestor<'a>(
    start: Entity,
    parents: &Query<&ChildOf>,
    dialogs: &'a Query<(Entity, &DialogVisual), (With<EditorDialog>, Without<DespawningDialog>)>,
) -> Option<(Entity, &'a DialogVisual)> {
    let mut current = start;
    loop {
        if let Ok((entity, visual)) = dialogs.get(current) {
            return Some((entity, visual));
        }
        let Ok(child_of) = parents.get(current) else {
            return None;
        };
        current = child_of.parent();
    }
}

fn handle_dialog_despawn(
    mut commands: Commands,
    mut removed_visual: RemovedComponents<EasingComponent<DialogVisual>>,
    despawning: Query<Entity, With<DespawningDialog>>,
) {
    for entity in removed_visual.read() {
        if despawning.contains(entity) {
            commands.entity(entity).try_despawn();
        }
    }
}
