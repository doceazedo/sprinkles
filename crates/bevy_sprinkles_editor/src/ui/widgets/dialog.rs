use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;
use bevy_easings::{CustomComponentEase, EaseFunction, EasingComponent, EasingType, Lerp};

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
        .add_observer(handle_backdrop_click)
        .add_systems(
            Update,
            (
                bevy_easings::custom_ease_system::<(), DialogVisual>,
                sync_dialog_visual,
                sync_children_slot_visibility,
                handle_esc_key,
                handle_dialog_despawn,
            ),
        );
}

#[derive(Component, Default, Clone)]
pub struct EditorDialog;

#[derive(Component, Default, Clone)]
struct DialogBackdrop;

#[derive(Component, Default, Clone)]
struct DialogPanel;

#[derive(Component, Default, Clone)]
struct DialogCloseButton;

#[derive(Component, Default, Clone)]
struct DialogCancelButton;

#[derive(Component, Default, Clone)]
struct DialogActionButton;

#[derive(Component, Default, Clone)]
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

#[derive(Component, Default, Clone)]
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
struct BaseBgAlpha(f32);

#[derive(Component)]
struct DespawningDialog;

fn on_open_dialog(
    event: On<OpenDialogEvent>,
    mut commands: Commands,
    existing: Query<Entity, With<EditorDialog>>,
) {
    if !existing.is_empty() {
        return;
    }

    spawn_dialog(&mut commands, &event);
}

fn on_open_confirmation_dialog(
    event: On<OpenConfirmationDialogEvent>,
    mut commands: Commands,
    existing: Query<Entity, With<EditorDialog>>,
) {
    if !existing.is_empty() {
        return;
    }

    let dialog_event: OpenDialogEvent = event.event().into();
    spawn_dialog(&mut commands, &dialog_event);
}

fn spawn_dialog(commands: &mut Commands, event: &OpenDialogEvent) {
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

    let dialog = commands.spawn_scene(dialog_scene(event)).id();
    commands.entity(dialog).insert(
        start_visual
            .ease_to(
                end_visual,
                EaseFunction::QuinticOut,
                EasingType::Once {
                    duration: ANIMATION_DURATION,
                },
            )
            .with_original_value(),
    );
}

fn dialog_scene(event: &OpenDialogEvent) -> impl Scene {
    let variant = event.variant;
    let config = DialogConfig {
        close_on_click_outside: event.close_on_click_outside,
        close_on_esc: event.close_on_esc,
    };
    let max_width = event.max_width.unwrap_or(px(448));

    let mut panel_children: Vec<Box<dyn SceneList>> = Vec::new();
    if event.title.is_some() || event.description.is_some() {
        panel_children.push(Box::new(bsn_list! {
            @dialog_header(event.title.clone(), event.description.clone())
        }) as Box<dyn SceneList>);
    }
    panel_children.push(Box::new(bsn_list! {
        @dialog_children_slot(event.content_padding)
    }) as Box<dyn SceneList>);
    if event.action.is_some() || event.cancel.is_some() {
        panel_children.push(Box::new(bsn_list! {
            @dialog_footer(event.cancel.clone(), event.action.clone(), variant)
        }) as Box<dyn SceneList>);
    }
    if event.has_close_button {
        panel_children.push(Box::new(bsn_list! {
            @dialog_close()
        }) as Box<dyn SceneList>);
    }

    bsn! {
        EditorDialog
        variant
        config
        Node {
            width: percent(100),
            height: percent(100),
            position_type: { PositionType::Absolute },
        }
        GlobalZIndex(200)
        Pickable::IGNORE
        Children [
            DialogBackdrop
            Node {
                width: percent(100),
                height: percent(100),
                position_type: { PositionType::Absolute },
                justify_content: { JustifyContent::Center },
                align_items: { AlignItems::Center },
            }
            BackgroundColor({ Color::BLACK.with_alpha(0.0) })
            Children [
                DialogPanel
                Node {
                    width: percent(100),
                    max_width: { max_width },
                    border: { UiRect::all(px(1)) },
                    border_radius: { BorderRadius::all(px(6)) },
                    flex_direction: { FlexDirection::Column },
                }
                BackgroundColor({ BACKGROUND_COLOR.with_alpha(0.0) })
                BorderColor::all(BORDER_COLOR.with_alpha(0.0))
                UiTransform {
                    scale: Vec2::splat(0.9),
                }
                Children [ { panel_children } ]
            ]
        ]
    }
}

fn dialog_header(title: Option<String>, description: Option<String>) -> impl Scene {
    let mut texts: Vec<Box<dyn SceneList>> = Vec::new();
    if let Some(title) = title {
        texts.push(Box::new(bsn_list! {
            Text({ title })
            TextFont {
                font: { FontSourceTemplate::Handle(FONT_PATH.into()) },
                font_size: TEXT_SIZE_XL,
                weight: { FontWeight::SEMIBOLD },
            }
            TextColor({ TEXT_DISPLAY_COLOR.with_alpha(0.0) })
        }) as Box<dyn SceneList>);
    }
    if let Some(description) = description {
        texts.push(Box::new(bsn_list! {
            Text({ description })
            TextFont {
                font: { FontSourceTemplate::Handle(FONT_PATH.into()) },
                font_size: TEXT_SIZE_LG,
            }
            TextColor({ TEXT_MUTED_COLOR.with_alpha(0.0) })
        }) as Box<dyn SceneList>);
    }

    bsn! {
        Node {
            padding: { UiRect::all(px(24)) },
            border: { UiRect::bottom(px(1)) },
            flex_direction: { FlexDirection::Column },
            row_gap: px(6),
        }
        BorderColor::all(BORDER_COLOR.with_alpha(0.0))
        Children [ { texts } ]
    }
}

fn dialog_children_slot(content_padding: UiRect) -> impl Scene {
    bsn! {
        DialogChildrenSlot
        Node {
            display: { Display::None },
            padding: { content_padding },
            border: { UiRect::bottom(px(1)) },
            flex_direction: { FlexDirection::Column },
            row_gap: px(12),
        }
        BorderColor::all(BORDER_COLOR.with_alpha(0.0))
    }
}

fn dialog_footer(
    cancel: Option<String>,
    action: Option<String>,
    variant: DialogVariant,
) -> impl Scene {
    let mut buttons: Vec<Box<dyn SceneList>> = Vec::new();
    if let Some(cancel) = cancel {
        buttons.push(Box::new(bsn_list! {
            DialogCancelButton
            @button(ButtonProps::new(cancel))
        }) as Box<dyn SceneList>);
    }
    if let Some(action) = action {
        let action_variant = variant.action_button_variant();
        buttons.push(Box::new(bsn_list! {
            DialogActionButton
            @button(ButtonProps::new(action).with_variant(action_variant))
        }) as Box<dyn SceneList>);
    }

    bsn! {
        Node {
            padding: { UiRect::all(px(24)) },
            column_gap: px(6),
            justify_content: { JustifyContent::End },
        }
        Children [ { buttons } ]
    }
}

fn dialog_close() -> impl Scene {
    bsn! {
        Node {
            position_type: { PositionType::Absolute },
            top: px(20),
            right: px(20),
        }
        Children [
            DialogCloseButton
            @icon_button(IconButtonProps::new(ICON_CLOSE).variant(ButtonVariant::Ghost))
        ]
    }
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
    event: On<PointerPress>,
    backdrops: Query<&ChildOf, With<DialogBackdrop>>,
    dialogs: Query<(&DialogConfig, &DialogVisual), (With<EditorDialog>, Without<DespawningDialog>)>,
    mut commands: Commands,
) {
    if event.entity != event.original_event_target() {
        return;
    }

    let Ok(child_of) = backdrops.get(event.entity) else {
        return;
    };

    let Ok((config, visual)) = dialogs.get(child_of.parent()) else {
        return;
    };

    if config.close_on_click_outside {
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
