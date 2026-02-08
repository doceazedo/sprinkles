use std::time::Duration;

use bevy::prelude::*;
use bevy_easings::{CustomComponentEase, EaseFunction, EasingComponent, EasingType, Lerp};

use bevy_ui_text_input::TextInputPrompt;

use crate::ui::tokens::{
    BACKGROUND_COLOR, BORDER_COLOR, FONT_PATH, TEXT_DISPLAY_COLOR, TEXT_MUTED_COLOR, TEXT_SIZE_LG,
    TEXT_SIZE_XL,
};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonVariant, EditorButton, IconButtonProps, button,
    icon_button,
};

const ICON_CLOSE: &str = "icons/ri-close-fill.png";
const ANIMATION_DURATION: Duration = Duration::from_millis(200);
const DIALOG_ANIMATION_OFFSET: f32 = 12.0;
const BACKDROP_TARGET_OPACITY: f32 = 0.8;

pub fn plugin(app: &mut App) {
    app.add_observer(on_open_dialog)
        .add_observer(on_open_confirmation_dialog)
        .add_observer(on_action_button_click)
        .add_observer(on_cancel_button_click)
        .add_observer(on_close_button_click)
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
pub struct OpenDialogEvent {
    pub title: Option<String>,
    pub description: Option<String>,
    pub action: Option<String>,
    pub cancel: Option<String>,
    pub variant: DialogVariant,
    pub has_close_button: bool,
    pub close_on_click_outside: bool,
    pub close_on_esc: bool,
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
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }

    pub fn with_cancel(mut self, cancel: impl Into<String>) -> Self {
        self.cancel = Some(cancel.into());
        self
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

    pub fn with_close_on_esc(mut self, close_on_esc: bool) -> Self {
        self.close_on_esc = close_on_esc;
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

fn spawn_dialog(
    commands: &mut Commands,
    asset_server: &AssetServer,
    event: &OpenDialogEvent,
) {
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
                    ButtonProps::new(action)
                        .with_variant(event.variant.action_button_variant()),
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
            max_width: px(448),
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
            padding: UiRect::all(px(24)),
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

fn sync_dialog_visual(
    dialogs: Query<(&DialogVisual, &Children), Changed<DialogVisual>>,
    mut commands: Commands,
    mut transforms: Query<&mut UiTransform>,
    mut bg_colors: Query<&mut BackgroundColor>,
    mut border_colors: Query<&mut BorderColor>,
    mut text_colors: Query<&mut TextColor>,
    mut image_nodes: Query<&mut ImageNode>,
    mut prompts: Query<(Entity, &mut TextInputPrompt)>,
    base_alphas: Query<&PromptBaseAlpha>,
    children_query: Query<&Children>,
    backdrop_query: Query<Entity, With<DialogBackdrop>>,
    buttons: Query<(&ButtonVariant, &Children), With<EditorButton>>,
) {
    if !dialogs.is_empty() {
        for (entity, prompt) in &prompts {
            if base_alphas.contains(entity) {
                continue;
            }
            if let Some(color) = prompt.color {
                let base: Srgba = color.into();
                commands.entity(entity).insert(PromptBaseAlpha(base.alpha));
            }
        }
    }

    for (visual, dialog_children) in &dialogs {
        let alpha = visual.opacity;

        for child in dialog_children.iter() {
            if !backdrop_query.contains(child) {
                continue;
            }

            if let Ok(mut bg) = bg_colors.get_mut(child) {
                bg.0 = Color::BLACK.with_alpha(BACKDROP_TARGET_OPACITY * alpha);
            }

            let Ok(backdrop_children) = children_query.get(child) else {
                continue;
            };

            for panel in backdrop_children.iter() {
                sync_panel_visual(
                    panel,
                    visual,
                    alpha,
                    &mut transforms,
                    &mut bg_colors,
                    &mut border_colors,
                    &mut text_colors,
                    &mut image_nodes,
                    &mut prompts,
                    &base_alphas,
                    &children_query,
                    &buttons,
                );
            }
        }
    }
}

fn sync_panel_visual(
    panel: Entity,
    visual: &DialogVisual,
    alpha: f32,
    transforms: &mut Query<&mut UiTransform>,
    bg_colors: &mut Query<&mut BackgroundColor>,
    border_colors: &mut Query<&mut BorderColor>,
    text_colors: &mut Query<&mut TextColor>,
    image_nodes: &mut Query<&mut ImageNode>,
    prompts: &mut Query<(Entity, &mut TextInputPrompt)>,
    base_alphas: &Query<&PromptBaseAlpha>,
    children_query: &Query<&Children>,
    buttons: &Query<(&ButtonVariant, &Children), With<EditorButton>>,
) {
    if let Ok(mut transform) = transforms.get_mut(panel) {
        transform.scale = visual.scale;
        transform.translation.y = px(visual.offset_y);
    }

    if let Ok(mut bg) = bg_colors.get_mut(panel) {
        bg.0 = BACKGROUND_COLOR.with_alpha(alpha).into();
    }
    if let Ok(mut border) = border_colors.get_mut(panel) {
        *border = BorderColor::all(BORDER_COLOR.with_alpha(alpha));
    }

    apply_alpha_recursive(
        panel,
        alpha,
        children_query,
        text_colors,
        image_nodes,
        bg_colors,
        border_colors,
        prompts,
        base_alphas,
        buttons,
    );
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

fn apply_button_alpha(
    entity: Entity,
    alpha: f32,
    variant: &ButtonVariant,
    button_children: &Children,
    bg_colors: &mut Query<&mut BackgroundColor>,
    border_colors: &mut Query<&mut BorderColor>,
    text_colors: &mut Query<&mut TextColor>,
    image_nodes: &mut Query<&mut ImageNode>,
) {
    if let Ok(mut bg) = bg_colors.get_mut(entity) {
        bg.0 = variant
            .bg_color(false)
            .with_alpha(variant.bg_opacity(false) * alpha)
            .into();
    }
    if let Ok(mut border) = border_colors.get_mut(entity) {
        *border = BorderColor::all(
            variant
                .border_color()
                .with_alpha(variant.border_opacity(false) * alpha),
        );
    }
    for btn_child in button_children.iter() {
        if let Ok(mut text_color) = text_colors.get_mut(btn_child) {
            text_color.0 = variant.text_color().with_alpha(alpha).into();
        }
        if let Ok(mut image) = image_nodes.get_mut(btn_child) {
            image.color = variant.text_color().with_alpha(alpha).into();
        }
    }
}

fn apply_alpha_recursive(
    entity: Entity,
    alpha: f32,
    children_query: &Query<&Children>,
    text_colors: &mut Query<&mut TextColor>,
    image_nodes: &mut Query<&mut ImageNode>,
    bg_colors: &mut Query<&mut BackgroundColor>,
    border_colors: &mut Query<&mut BorderColor>,
    prompts: &mut Query<(Entity, &mut TextInputPrompt)>,
    base_alphas: &Query<&PromptBaseAlpha>,
    buttons: &Query<(&ButtonVariant, &Children), With<EditorButton>>,
) {
    if let Ok((variant, button_children)) = buttons.get(entity) {
        apply_button_alpha(
            entity,
            alpha,
            variant,
            button_children,
            bg_colors,
            border_colors,
            text_colors,
            image_nodes,
        );
        return;
    }

    if let Ok(mut text_color) = text_colors.get_mut(entity) {
        let base: Srgba = text_color.0.into();
        text_color.0 = base.with_alpha(alpha).into();
    }

    if let Ok(mut image) = image_nodes.get_mut(entity) {
        let base: Srgba = image.color.into();
        image.color = base.with_alpha(alpha).into();
    }

    if let Ok(mut border) = border_colors.get_mut(entity) {
        let base: Srgba = border.top.into();
        *border = BorderColor::all(base.with_alpha(alpha));
    }

    if let Ok((_, mut prompt)) = prompts.get_mut(entity) {
        if let Some(color) = &mut prompt.color {
            let base: Srgba = (*color).into();
            let base_alpha = base_alphas.get(entity).map(|b| b.0).unwrap_or(base.alpha);
            *color = base.with_alpha(base_alpha * alpha).into();
        }
    }

    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            apply_alpha_recursive(
                child,
                alpha,
                children_query,
                text_colors,
                image_nodes,
                bg_colors,
                border_colors,
                prompts,
                base_alphas,
                buttons,
            );
        }
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
    dialogs: Query<(Entity, &DialogConfig, &DialogVisual), (With<EditorDialog>, Without<DespawningDialog>)>,
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
