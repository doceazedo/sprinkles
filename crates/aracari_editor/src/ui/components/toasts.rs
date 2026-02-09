use crate::ui::tokens::TEXT_BODY_COLOR;
use crate::ui::widgets::button::{ButtonClickEvent, EditorButton};
use crate::ui::widgets::separator::EditorSeparator;
use crate::ui::widgets::toast::{
    DEFAULT_TOAST_DURATION, EditorToast, TOAST_BOTTOM_OFFSET, ToastCloseButton, ToastDuration,
    ToastIcon, ToastText, ToastVariant, toast,
};
use bevy::prelude::*;
use bevy_easings::{CustomComponentEase, EaseFunction, EasingComponent, EasingType, Lerp};
use std::time::Duration;

#[derive(Component, Clone)]
struct ToastVisual {
    scale: Vec2,
    opacity: f32,
    offset_y: f32,
}

impl Default for ToastVisual {
    fn default() -> Self {
        Self {
            scale: Vec2::ONE,
            opacity: 1.0,
            offset_y: 0.0,
        }
    }
}

impl Lerp for ToastVisual {
    type Scalar = f32;

    fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
        Self {
            scale: self.scale.lerp(other.scale, *scalar),
            opacity: self.opacity.lerp(other.opacity, *scalar),
            offset_y: self.offset_y.lerp(other.offset_y, *scalar),
        }
    }
}

macro_rules! impl_f32_lerp {
    ($($ty:ident),*) => {
        $(
            impl Lerp for $ty {
                type Scalar = f32;
                fn lerp(&self, other: &Self, scalar: &Self::Scalar) -> Self {
                    Self(self.0.lerp(other.0, *scalar))
                }
            }
        )*
    };
}

#[derive(Component, Clone, Default)]
struct ToastPosition(f32);

#[derive(Component, Clone)]
struct ToastStackScale(f32);

impl Default for ToastStackScale {
    fn default() -> Self {
        Self(1.0)
    }
}

impl_f32_lerp!(ToastPosition, ToastStackScale);

const ANIMATION_DURATION: Duration = Duration::from_millis(200);
const TOAST_HEIGHT: f32 = 48.0;
const TOAST_ANIMATION_OFFSET: f32 = 12.0;
const STACKED_OFFSET: f32 = 12.0;
const STACKED_SCALE: f32 = 0.9;
const MAX_TOASTS: usize = 5;

#[derive(Component)]
struct ToastIndex(usize);

#[derive(Resource, Default)]
struct ToastsExpanded(bool);

pub fn plugin(app: &mut App) {
    app.init_resource::<ToastsExpanded>()
        .add_observer(on_toast_event)
        .add_observer(on_dismiss_toast_event)
        .add_observer(on_close_button_click)
        .add_systems(Startup, setup_toasts_container)
        .add_systems(
            Update,
            (
                bevy_easings::custom_ease_system::<(), ToastVisual>,
                bevy_easings::custom_ease_system::<(), ToastPosition>,
                bevy_easings::custom_ease_system::<(), ToastStackScale>,
                sync_toast_visual,
                sync_toast_position,
                sync_toast_stack_scale,
                setup_toast_close_buttons,
                update_hitbox_size,
                update_expanded_state,
                update_toast_stacking,
                handle_toast_timer,
                handle_toast_despawn,
            ),
        );
}

fn sync_toast_visual(
    toasts: Query<(Entity, &ToastVisual, &ToastVariant, &Children), Changed<ToastVisual>>,
    children_query: Query<&Children>,
    mut bg_colors: Query<&mut BackgroundColor>,
    mut border_colors: Query<&mut BorderColor>,
    mut text_colors: Query<&mut TextColor>,
    mut image_nodes: Query<&mut ImageNode>,
    icons: Query<Entity, With<ToastIcon>>,
    texts: Query<Entity, With<ToastText>>,
    separators: Query<Entity, With<EditorSeparator>>,
    buttons: Query<Entity, With<EditorButton>>,
) {
    for (toast_entity, visual, variant, _) in &toasts {
        let alpha = visual.opacity;

        if let Ok(mut bg) = bg_colors.get_mut(toast_entity) {
            bg.0 = variant.bg_color().with_alpha(alpha).into();
        }

        if let Ok(mut border) = border_colors.get_mut(toast_entity) {
            let border_color = variant
                .bg_color()
                .mix((&TEXT_BODY_COLOR).into(), 0.1 * alpha);
            *border = BorderColor::all(border_color);
        }

        apply_opacity_recursive(
            toast_entity,
            alpha,
            &children_query,
            &mut text_colors,
            &mut image_nodes,
            &mut bg_colors,
            &icons,
            &texts,
            &separators,
            &buttons,
        );
    }
}

fn sync_toast_position(mut toasts: Query<(&ToastPosition, &mut Node), Changed<ToastPosition>>) {
    for (position, mut node) in &mut toasts {
        node.bottom = Val::Px(TOAST_BOTTOM_OFFSET + position.0);
    }
}

fn sync_toast_stack_scale(
    mut toasts: Query<
        (&ToastVisual, &ToastStackScale, &mut UiTransform),
        Or<(Changed<ToastStackScale>, Changed<ToastVisual>)>,
    >,
) {
    for (visual, stack_scale, mut ui_transform) in &mut toasts {
        ui_transform.scale = visual.scale * stack_scale.0;
        ui_transform.translation.y = Val::Px(visual.offset_y);
    }
}

fn apply_opacity_recursive(
    entity: Entity,
    alpha: f32,
    children_query: &Query<&Children>,
    text_colors: &mut Query<&mut TextColor>,
    image_nodes: &mut Query<&mut ImageNode>,
    bg_colors: &mut Query<&mut BackgroundColor>,
    icons: &Query<Entity, With<ToastIcon>>,
    texts: &Query<Entity, With<ToastText>>,
    separators: &Query<Entity, With<EditorSeparator>>,
    buttons: &Query<Entity, With<EditorButton>>,
) {
    if icons.contains(entity) {
        if let Ok(mut image) = image_nodes.get_mut(entity) {
            image.color = TEXT_BODY_COLOR.with_alpha(alpha).into();
        }
    }

    if texts.contains(entity) {
        if let Ok(mut text_color) = text_colors.get_mut(entity) {
            text_color.0 = TEXT_BODY_COLOR.with_alpha(alpha).into();
        }
    }

    if separators.contains(entity) {
        if let Ok(mut bg) = bg_colors.get_mut(entity) {
            bg.0 = TEXT_BODY_COLOR.with_alpha(0.1 * alpha).into();
        }
    }

    if buttons.contains(entity) {
        if let Ok(children) = children_query.get(entity) {
            for child in children.iter() {
                if let Ok(mut image) = image_nodes.get_mut(child) {
                    image.color = TEXT_BODY_COLOR.with_alpha(alpha).into();
                }
            }
        }
    }

    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            apply_opacity_recursive(
                child,
                alpha,
                children_query,
                text_colors,
                image_nodes,
                bg_colors,
                icons,
                texts,
                separators,
                buttons,
            );
        }
    }
}

#[derive(Component)]
pub struct ToastsContainer;

#[derive(Component)]
struct ToastsHitbox;

#[derive(Component)]
struct DespawningToast;

#[derive(Event)]
pub struct ToastEvent {
    pub variant: ToastVariant,
    pub content: String,
    pub duration: Duration,
}

impl ToastEvent {
    fn new(variant: ToastVariant, content: impl Into<String>) -> Self {
        Self {
            variant,
            content: content.into(),
            duration: DEFAULT_TOAST_DURATION,
        }
    }

    pub fn info(content: impl Into<String>) -> Self {
        Self::new(ToastVariant::Info, content)
    }

    pub fn success(content: impl Into<String>) -> Self {
        Self::new(ToastVariant::Success, content)
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self::new(ToastVariant::Error, content)
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
}

#[derive(EntityEvent)]
pub struct DismissToastEvent {
    pub entity: Entity,
}

fn setup_toasts_container(mut commands: Commands) {
    commands
        .spawn((
            ToastsContainer,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            GlobalZIndex(100),
            Pickable::IGNORE,
        ))
        .with_children(|parent| {
            parent.spawn((
                ToastsHitbox,
                Interaction::None,
                Pickable {
                    should_block_lower: false,
                    is_hoverable: true,
                },
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    bottom: Val::Px(TOAST_BOTTOM_OFFSET),
                    width: Val::Px(0.0),
                    height: Val::Px(0.0),
                    ..default()
                },
                UiTransform {
                    translation: Val2 {
                        x: Val::Percent(-50.0),
                        y: Val::Px(0.0),
                    },
                    ..default()
                },
                ZIndex(10),
            ));
        });
}

fn on_toast_event(
    event: On<ToastEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    container: Query<Entity, With<ToastsContainer>>,
    mut existing_toasts: Query<
        (Entity, &mut ToastIndex),
        (With<EditorToast>, Without<DespawningToast>),
    >,
) {
    let Ok(container_entity) = container.single() else {
        return;
    };

    for (entity, mut index) in &mut existing_toasts {
        index.0 += 1;
        if index.0 >= MAX_TOASTS {
            commands.trigger(DismissToastEvent { entity });
        }
    }

    let start_visual = ToastVisual {
        scale: Vec2::splat(0.75),
        opacity: 0.0,
        offset_y: TOAST_ANIMATION_OFFSET,
    };

    let end_visual = ToastVisual {
        scale: Vec2::ONE,
        opacity: 1.0,
        offset_y: 0.0,
    };

    let toast_entity = commands
        .spawn((
            toast(event.variant, &event.content, event.duration, &asset_server),
            ToastIndex(0),
            ToastPosition(0.0),
            ToastStackScale(1.0),
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
        .id();

    commands.entity(container_entity).add_child(toast_entity);
}

fn on_dismiss_toast_event(
    event: On<DismissToastEvent>,
    mut commands: Commands,
    toasts: Query<(Entity, &ToastVisual), (With<EditorToast>, Without<DespawningToast>)>,
) {
    let Ok((_, current_visual)) = toasts.get(event.entity) else {
        return;
    };

    let end_visual = ToastVisual {
        scale: Vec2::splat(0.75),
        opacity: 0.0,
        offset_y: current_visual.offset_y + TOAST_ANIMATION_OFFSET,
    };

    commands.entity(event.entity).insert((
        DespawningToast,
        current_visual
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

fn setup_toast_close_buttons(
    mut commands: Commands,
    new_toasts: Query<(Entity, &Children), Added<EditorToast>>,
    children_query: Query<&Children>,
    buttons: Query<Entity, With<Button>>,
) {
    for (toast_entity, children) in &new_toasts {
        for child in children.iter() {
            if let Some(button_entity) = find_button_recursive(child, &children_query, &buttons) {
                commands
                    .entity(button_entity)
                    .insert(ToastCloseButton(toast_entity));
            }
        }
    }
}

fn find_button_recursive(
    entity: Entity,
    children_query: &Query<&Children>,
    buttons: &Query<Entity, With<Button>>,
) -> Option<Entity> {
    if buttons.get(entity).is_ok() {
        return Some(entity);
    }

    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            if let Some(found) = find_button_recursive(child, children_query, buttons) {
                return Some(found);
            }
        }
    }

    None
}

fn on_close_button_click(
    event: On<ButtonClickEvent>,
    close_buttons: Query<&ToastCloseButton>,
    mut commands: Commands,
) {
    let Ok(close_button) = close_buttons.get(event.entity) else {
        return;
    };

    commands.trigger(DismissToastEvent {
        entity: close_button.0,
    });
}

fn update_hitbox_size(
    expanded: Res<ToastsExpanded>,
    toasts: Query<&ComputedNode, (With<EditorToast>, Without<DespawningToast>)>,
    mut hitbox: Query<&mut Node, With<ToastsHitbox>>,
) {
    let Ok(mut hitbox_node) = hitbox.single_mut() else {
        return;
    };

    let toast_count = toasts.iter().count();
    if toast_count == 0 {
        hitbox_node.width = Val::Px(0.0);
        hitbox_node.height = Val::Px(0.0);
        return;
    }

    let max_width = toasts
        .iter()
        .map(|computed| computed.size().x * computed.inverse_scale_factor)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);

    let height = if expanded.0 {
        TOAST_HEIGHT * toast_count as f32
    } else {
        TOAST_HEIGHT + STACKED_OFFSET * (toast_count - 1) as f32
    };

    hitbox_node.width = Val::Px(max_width);
    hitbox_node.height = Val::Px(height);
}

fn update_expanded_state(
    hitbox: Query<&Interaction, With<ToastsHitbox>>,
    mut expanded: ResMut<ToastsExpanded>,
) {
    let Ok(interaction) = hitbox.single() else {
        return;
    };

    let is_hovered = matches!(interaction, Interaction::Hovered | Interaction::Pressed);
    if expanded.0 != is_hovered {
        expanded.0 = is_hovered;
    }
}

fn update_toast_stacking(
    mut commands: Commands,
    expanded: Res<ToastsExpanded>,
    new_toasts: Query<Entity, Added<EditorToast>>,
    despawning_toasts: Query<Entity, Added<DespawningToast>>,
    toasts: Query<
        (Entity, &ToastIndex, &ToastPosition, &ToastStackScale),
        (With<EditorToast>, Without<DespawningToast>),
    >,
) {
    let has_new_toast = !new_toasts.is_empty();
    let has_despawning = !despawning_toasts.is_empty();

    if !expanded.is_changed() && !has_new_toast && !has_despawning {
        return;
    }

    let mut toast_list: Vec<_> = toasts.iter().collect();
    toast_list.sort_by_key(|(_, index, _, _)| index.0);

    for (i, (entity, _, current_pos, current_scale)) in toast_list.into_iter().enumerate() {
        let (target_pos, target_scale) = if expanded.0 {
            (i as f32 * TOAST_HEIGHT, 1.0)
        } else {
            (i as f32 * STACKED_OFFSET, STACKED_SCALE.powi(i as i32))
        };

        if (current_pos.0 - target_pos).abs() > 0.1 {
            commands.entity(entity).insert(
                current_pos
                    .clone()
                    .ease_to(
                        ToastPosition(target_pos),
                        EaseFunction::QuinticOut,
                        EasingType::Once {
                            duration: ANIMATION_DURATION,
                        },
                    )
                    .with_original_value(),
            );
        }

        if i > 0 && (current_scale.0 - target_scale).abs() > 0.01 {
            commands.entity(entity).insert(
                current_scale
                    .clone()
                    .ease_to(
                        ToastStackScale(target_scale),
                        EaseFunction::QuinticOut,
                        EasingType::Once {
                            duration: ANIMATION_DURATION,
                        },
                    )
                    .with_original_value(),
            );
        }
    }
}

fn handle_toast_timer(
    mut commands: Commands,
    time: Res<Time>,
    mut toasts: Query<
        (Entity, &ToastIndex, &mut ToastDuration),
        (With<EditorToast>, Without<DespawningToast>),
    >,
    expanded: Res<ToastsExpanded>,
) {
    for (entity, index, mut duration) in &mut toasts {
        if expanded.0 && index.0 == 0 {
            continue;
        }

        duration.0.tick(time.delta());
        if duration.0.just_finished() {
            commands.trigger(DismissToastEvent { entity });
        }
    }
}

fn handle_toast_despawn(
    mut commands: Commands,
    mut removed_visual: RemovedComponents<EasingComponent<ToastVisual>>,
    despawning: Query<Entity, With<DespawningToast>>,
) {
    for entity in removed_visual.read() {
        if despawning.contains(entity) {
            commands.entity(entity).despawn();
        }
    }
}
