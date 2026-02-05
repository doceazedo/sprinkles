use aracari::prelude::{CurveEasing, CurveMode, CurvePoint, CurveTexture};
use bevy::input_focus::InputFocus;
use bevy::picking::events::{Press, Release};
use bevy::picking::pointer::PointerButton;
use bevy::picking::hover::Hovered;
use bevy::picking::prelude::Pickable;
use bevy::prelude::*;
use bevy::window::{CursorIcon, SystemCursorIcon};
use bevy::reflect::{TypePath, Typed};
use bevy::render::render_resource::*;
use bevy::shader::ShaderRef;
use bevy::ui::UiGlobalTransform;
use inflector::Inflector;
use crate::ui::tokens::{BACKGROUND_COLOR, BORDER_COLOR, FONT_PATH, PRIMARY_COLOR, TEXT_MUTED_COLOR, TEXT_SIZE_SM};
use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonVariant, IconButtonProps, button, icon_button, set_button_variant,
};
use crate::ui::widgets::combobox::{
    ComboBoxChangeEvent, ComboBoxOptionData, combobox_with_label,
};
use crate::ui::widgets::popover::{
    EditorPopover, PopoverHeaderProps, PopoverPlacement, PopoverProps, popover, popover_content,
    popover_header,
};
use crate::ui::widgets::text_edit::EditorTextEdit;
use crate::ui::widgets::utils::is_descendant_of;
use crate::ui::widgets::vector_edit::{EditorVectorEdit, VectorEditProps, VectorSize, VectorSuffixes, vector_edit};
use bevy_ui_text_input::TextInputQueue;
use bevy_ui_text_input::actions::{TextInputAction, TextInputEdit};

const ICON_CURVE: &str = "icons/blender_fcurve.png";
const ICON_MORE: &str = "icons/ri-more-fill.png";
const ICON_FLIP: &str = "icons/ri-arrow-left-right-fill.png";
const SHADER_CURVE_PATH: &str = "shaders/curve_edit.wgsl";
const CANVAS_SIZE: f32 = 232.0;
const CONTENT_PADDING: f32 = 12.0;
const POINT_HANDLE_SIZE: f32 = 12.0;
const TENSION_HANDLE_SIZE: f32 = 10.0;
const HANDLE_BORDER: f32 = 1.0;
const BORDER_RADIUS: f32 = 4.0;
const MAX_POINTS: usize = 8;
const DRAG_SNAP_STEP: f64 = 0.01;

pub fn plugin(app: &mut App) {
    app.add_plugins(UiMaterialPlugin::<CurveMaterial>::default())
        .add_observer(handle_trigger_click)
        .add_observer(handle_preset_change)
        .add_observer(handle_flip_click)
        .add_observer(handle_point_mode_change)
        .add_systems(
            Update,
            (
                setup_curve_edit,
                setup_curve_edit_content,
                update_curve_visuals,
                respawn_handles_on_point_change,
                update_handle_cursors,
                update_handle_colors,
                handle_popover_closed,
                sync_trigger_label,
                sync_range_inputs_to_state,
                handle_range_blur,
                handle_canvas_right_click,
                handle_point_right_click,
                handle_tension_right_click,
            ),
        );
}

#[derive(Component)]
pub struct EditorCurveEdit;

#[derive(Component, Clone)]
pub struct CurveEditState {
    pub curve: CurveTexture,
    pub preset_name: Option<String>,
    popover: Option<Entity>,
}

impl Default for CurveEditState {
    fn default() -> Self {
        Self {
            curve: CurveTexture::default(),
            preset_name: Some("Constant".to_string()),
            popover: None,
        }
    }
}

impl CurveEditState {
    pub fn from_curve(curve: CurveTexture) -> Self {
        let preset_name = detect_preset(&curve);
        Self {
            curve,
            preset_name,
            popover: None,
        }
    }

    pub fn set_curve(&mut self, curve: CurveTexture) {
        self.curve = curve;
        self.preset_name = detect_preset(&self.curve);
    }

    pub fn mark_custom(&mut self) {
        self.preset_name = None;
    }

    pub fn label(&self) -> &str {
        self.preset_name.as_deref().unwrap_or("Custom curve")
    }
}

fn detect_preset(curve: &CurveTexture) -> Option<String> {
    if curve.points.len() == 2 {
        let p0 = &curve.points[0];
        let p1 = &curve.points[1];

        let is_zero_to_one = (p0.position - 0.0).abs() < f32::EPSILON
            && (p1.position - 1.0).abs() < f32::EPSILON
            && (p0.value - 0.0).abs() < f64::EPSILON
            && (p1.value - 1.0).abs() < f64::EPSILON;

        let is_constant = (p0.position - 0.0).abs() < f32::EPSILON
            && (p1.position - 1.0).abs() < f32::EPSILON
            && (p0.value - 1.0).abs() < f64::EPSILON
            && (p1.value - 1.0).abs() < f64::EPSILON;

        if is_constant {
            return Some("Constant".to_string());
        }

        if is_zero_to_one {
            // tension values for power-based easings
            const QUAD_TENSION: f64 = 0.5005;
            const CUBIC_TENSION: f64 = 0.6673;
            const QUART_TENSION: f64 = 0.7507;
            const QUINT_TENSION: f64 = 0.8008;
            const TOLERANCE: f64 = 0.01;

            match (p1.mode, p1.easing) {
                (CurveMode::DoubleCurve, CurveEasing::Power) => {
                    let t = p1.tension.abs();
                    if t < TOLERANCE {
                        return Some("Linear".to_string());
                    } else if (t - QUAD_TENSION).abs() < TOLERANCE {
                        return Some("Quad in out".to_string());
                    } else if (t - CUBIC_TENSION).abs() < TOLERANCE {
                        return Some("Cubic in out".to_string());
                    } else if (t - QUART_TENSION).abs() < TOLERANCE {
                        return Some("Quart in out".to_string());
                    } else if (t - QUINT_TENSION).abs() < TOLERANCE {
                        return Some("Quint in out".to_string());
                    }
                }
                (CurveMode::SingleCurve, CurveEasing::Power) => {
                    let t = p1.tension.abs();
                    let suffix = if p1.tension >= 0.0 { "in" } else { "out" };
                    if (t - QUAD_TENSION).abs() < TOLERANCE {
                        return Some(format!("Quad {}", suffix));
                    } else if (t - CUBIC_TENSION).abs() < TOLERANCE {
                        return Some(format!("Cubic {}", suffix));
                    } else if (t - QUART_TENSION).abs() < TOLERANCE {
                        return Some(format!("Quart {}", suffix));
                    } else if (t - QUINT_TENSION).abs() < TOLERANCE {
                        return Some(format!("Quint {}", suffix));
                    }
                }
                (CurveMode::SingleCurve, CurveEasing::Sine) => {
                    if p1.tension >= 0.0 {
                        return Some("Sine in".to_string());
                    } else {
                        return Some("Sine out".to_string());
                    }
                }
                (CurveMode::DoubleCurve, CurveEasing::Sine) => {
                    return Some("Sine in out".to_string());
                }
                (CurveMode::SingleCurve, CurveEasing::Expo) => {
                    if p1.tension >= 0.0 {
                        return Some("Expo in".to_string());
                    } else {
                        return Some("Expo out".to_string());
                    }
                }
                (CurveMode::DoubleCurve, CurveEasing::Expo) => {
                    return Some("Expo in out".to_string());
                }
                (CurveMode::SingleCurve, CurveEasing::Circ) => {
                    if p1.tension >= 0.0 {
                        return Some("Circ in".to_string());
                    } else {
                        return Some("Circ out".to_string());
                    }
                }
                (CurveMode::DoubleCurve, CurveEasing::Circ) => {
                    return Some("Circ in out".to_string());
                }
                _ => {}
            }
        }
    }
    None
}

#[derive(EntityEvent)]
pub struct CurveEditChangeEvent {
    pub entity: Entity,
    pub curve: CurveTexture,
}

#[derive(EntityEvent)]
pub struct CurveEditCommitEvent {
    pub entity: Entity,
    pub curve: CurveTexture,
}

#[derive(Component)]
struct CurveEditConfig {
    initialized: bool,
}

#[derive(Default)]
pub struct CurveEditProps {
    pub curve: Option<CurveTexture>,
    pub label: Option<String>,
}

impl CurveEditProps {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_curve(mut self, curve: CurveTexture) -> Self {
        self.curve = Some(curve);
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

pub fn curve_edit(props: CurveEditProps) -> impl Bundle {
    let CurveEditProps { curve, label: _ } = props;

    let state = curve
        .map(CurveEditState::from_curve)
        .unwrap_or_default();

    (
        EditorCurveEdit,
        state,
        CurveEditConfig { initialized: false },
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(3.0),
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: px(0.0),
            ..default()
        },
    )
}

#[derive(Component)]
struct CurveEditTrigger(Entity);

#[derive(Component)]
struct CurveEditTriggerLabel(Entity);

#[derive(Component)]
struct CurveEditPopover(Entity);

#[derive(Component)]
struct CurveEditContent(Entity);

#[derive(Component)]
struct CurveCanvas {
    curve_edit: Entity,
    point_count: usize,
}

#[derive(Component)]
struct CurveMaterialNode(Entity);

#[derive(Component)]
struct PresetComboBox(Entity);

#[derive(Component)]
struct FlipButton(Entity);

#[derive(Component)]
struct RangeEdit(Entity);

#[derive(Component)]
struct PointHandle {
    curve_edit: Entity,
    canvas: Entity,
    index: usize,
}

#[derive(Component)]
struct TensionHandle {
    curve_edit: Entity,
    canvas: Entity,
    index: usize,
}

#[derive(Component)]
struct PointModeMenu {
    curve_edit: Entity,
    index: usize,
}

#[derive(Component, Default)]
struct Dragging;

fn pack_f32(values: &[f32; MAX_POINTS]) -> [Vec4; 2] {
    [
        Vec4::new(values[0], values[1], values[2], values[3]),
        Vec4::new(values[4], values[5], values[6], values[7]),
    ]
}

fn pack_u32(values: &[u32; MAX_POINTS]) -> [UVec4; 2] {
    [
        UVec4::new(values[0], values[1], values[2], values[3]),
        UVec4::new(values[4], values[5], values[6], values[7]),
    ]
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone, Default)]
pub struct CurveMaterial {
    #[uniform(0)]
    border_radius: f32,
    #[uniform(0)]
    point_count: u32,
    #[uniform(0)]
    range_min: f32,
    #[uniform(0)]
    range_max: f32,
    #[uniform(0)]
    positions_low: Vec4,
    #[uniform(0)]
    positions_high: Vec4,
    #[uniform(0)]
    values_low: Vec4,
    #[uniform(0)]
    values_high: Vec4,
    #[uniform(0)]
    modes_low: UVec4,
    #[uniform(0)]
    modes_high: UVec4,
    #[uniform(0)]
    tensions_low: Vec4,
    #[uniform(0)]
    tensions_high: Vec4,
    #[uniform(0)]
    easings_low: UVec4,
    #[uniform(0)]
    easings_high: UVec4,
}

impl CurveMaterial {
    fn from_curve(curve: &CurveTexture) -> Self {
        let mut positions = [0.0f32; MAX_POINTS];
        let mut values = [0.0f32; MAX_POINTS];
        let mut modes = [0u32; MAX_POINTS];
        let mut tensions = [0.0f32; MAX_POINTS];
        let mut easings = [0u32; MAX_POINTS];

        for (i, point) in curve.points.iter().take(MAX_POINTS).enumerate() {
            positions[i] = point.position;
            values[i] = point.value as f32;
            modes[i] = point.mode as u32;
            tensions[i] = point.tension as f32;
            easings[i] = point.easing as u32;
        }

        let [positions_low, positions_high] = pack_f32(&positions);
        let [values_low, values_high] = pack_f32(&values);
        let [modes_low, modes_high] = pack_u32(&modes);
        let [tensions_low, tensions_high] = pack_f32(&tensions);
        let [easings_low, easings_high] = pack_u32(&easings);

        Self {
            border_radius: BORDER_RADIUS,
            point_count: curve.points.len().min(MAX_POINTS) as u32,
            range_min: curve.range.min,
            range_max: curve.range.max,
            positions_low,
            positions_high,
            values_low,
            values_high,
            modes_low,
            modes_high,
            tensions_low,
            tensions_high,
            easings_low,
            easings_high,
        }
    }
}

impl UiMaterial for CurveMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_CURVE_PATH.into()
    }
}

trait CurveControl: Component {
    fn curve_edit_entity(&self) -> Entity;
    fn canvas_entity(&self) -> Entity;
    fn update_state(&self, state: &mut CurveEditState, normalized: Vec2, delta: Option<Vec2>);
}

impl CurveControl for CurveCanvas {
    fn curve_edit_entity(&self) -> Entity {
        self.curve_edit
    }

    fn canvas_entity(&self) -> Entity {
        panic!("CurveCanvas should not be used as a control target")
    }

    fn update_state(&self, _state: &mut CurveEditState, _normalized: Vec2, _delta: Option<Vec2>) {}
}

impl CurveControl for PointHandle {
    fn curve_edit_entity(&self) -> Entity {
        self.curve_edit
    }

    fn canvas_entity(&self) -> Entity {
        self.canvas
    }

    fn update_state(&self, state: &mut CurveEditState, normalized: Vec2, _delta: Option<Vec2>) {
        if self.index >= state.curve.points.len() {
            return;
        }

        let new_pos = (normalized.x + 0.5).clamp(0.0, 1.0);
        let snapped_pos = (new_pos as f64 / DRAG_SNAP_STEP).round() * DRAG_SNAP_STEP;
        let prev_pos = if self.index > 0 {
            state.curve.points[self.index - 1].position + 0.001
        } else {
            0.0
        };
        let next_pos = if self.index < state.curve.points.len() - 1 {
            state.curve.points[self.index + 1].position - 0.001
        } else {
            1.0
        };
        let clamped_pos = (snapped_pos as f32).clamp(prev_pos, next_pos);

        let range_min = state.curve.range.min as f64;
        let range_max = state.curve.range.max as f64;
        let range_span = state.curve.range.span() as f64;
        let normalized_value = 0.5 - normalized.y;
        let raw_value = (range_min + normalized_value as f64 * range_span).clamp(range_min, range_max);
        let snapped_value = (raw_value / DRAG_SNAP_STEP).round() * DRAG_SNAP_STEP;

        state.curve.points[self.index].position = clamped_pos;
        state.curve.points[self.index].value = snapped_value;

        state.mark_custom();
    }
}

impl CurveControl for TensionHandle {
    fn curve_edit_entity(&self) -> Entity {
        self.curve_edit
    }

    fn canvas_entity(&self) -> Entity {
        self.canvas
    }

    fn update_state(&self, state: &mut CurveEditState, _normalized: Vec2, delta: Option<Vec2>) {
        if self.index == 0 || self.index >= state.curve.points.len() {
            return;
        }

        let Some(delta) = delta else {
            return;
        };

        let p1 = &state.curve.points[self.index];
        let mode = p1.mode;
        let current_tension = p1.tension;

        const TENSION_SENSITIVITY: f64 = 0.005;

        match mode {
            CurveMode::SingleCurve | CurveMode::DoubleCurve => {
                let tension_delta = -delta.y as f64 * TENSION_SENSITIVITY;
                let raw_tension = (current_tension + tension_delta).clamp(-1.0, 1.0);
                let snapped_tension = (raw_tension / DRAG_SNAP_STEP).round() * DRAG_SNAP_STEP;
                state.curve.points[self.index].tension = snapped_tension;
            }
            CurveMode::Stairs | CurveMode::SmoothStairs => {
                let tension_delta = -delta.y as f64 * TENSION_SENSITIVITY;
                let raw_tension = (current_tension + tension_delta).clamp(0.0, 1.0);
                let snapped_tension = (raw_tension / DRAG_SNAP_STEP).round() * DRAG_SNAP_STEP;
                state.curve.points[self.index].tension = snapped_tension;
            }
            CurveMode::Hold => {}
        }

        state.mark_custom();
    }
}

fn on_control_press<C: CurveControl>(
    event: On<Pointer<Press>>,
    mut commands: Commands,
    controls: Query<&C>,
    canvases: Query<(&ComputedNode, &UiGlobalTransform), With<CurveCanvas>>,
    mut states: Query<&mut CurveEditState>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok(control) = controls.get(event.event_target()) else {
        return;
    };
    let curve_edit_entity = control.curve_edit_entity();
    let canvas_entity = control.canvas_entity();

    let Ok((computed, ui_transform)) = canvases.get(canvas_entity) else {
        return;
    };

    let cursor_pos = event.pointer_location.position / computed.inverse_scale_factor;
    let Some(normalized) = computed.normalize_point(*ui_transform, cursor_pos) else {
        return;
    };

    let Ok(mut state) = states.get_mut(curve_edit_entity) else {
        return;
    };

    control.update_state(&mut state, normalized, None);

    commands.trigger(CurveEditChangeEvent {
        entity: curve_edit_entity,
        curve: state.curve.clone(),
    });
}

fn on_control_release<C: CurveControl>(
    event: On<Pointer<Release>>,
    mut commands: Commands,
    controls: Query<&C, Without<Dragging>>,
    states: Query<&CurveEditState>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok(control) = controls.get(event.event_target()) else {
        return;
    };
    let curve_edit_entity = control.curve_edit_entity();

    if let Ok(state) = states.get(curve_edit_entity) {
        commands.trigger(CurveEditCommitEvent {
            entity: curve_edit_entity,
            curve: state.curve.clone(),
        });
    }
}

fn on_control_drag_start<C: CurveControl>(
    event: On<Pointer<DragStart>>,
    mut commands: Commands,
    controls: Query<&C>,
    canvases: Query<(&ComputedNode, &UiGlobalTransform), With<CurveCanvas>>,
    mut states: Query<&mut CurveEditState>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok(control) = controls.get(event.event_target()) else {
        return;
    };
    let curve_edit_entity = control.curve_edit_entity();
    let canvas_entity = control.canvas_entity();

    commands.entity(event.event_target()).insert(Dragging);

    let Ok((computed, ui_transform)) = canvases.get(canvas_entity) else {
        return;
    };

    let cursor_pos = event.pointer_location.position / computed.inverse_scale_factor;
    let Some(normalized) = computed.normalize_point(*ui_transform, cursor_pos) else {
        return;
    };

    let Ok(mut state) = states.get_mut(curve_edit_entity) else {
        return;
    };

    control.update_state(&mut state, normalized, None);

    commands.trigger(CurveEditChangeEvent {
        entity: curve_edit_entity,
        curve: state.curve.clone(),
    });
}

fn on_control_drag<C: CurveControl>(
    event: On<Pointer<Drag>>,
    mut commands: Commands,
    controls: Query<&C, With<Dragging>>,
    canvases: Query<(&ComputedNode, &UiGlobalTransform), With<CurveCanvas>>,
    mut states: Query<&mut CurveEditState>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok(control) = controls.get(event.event_target()) else {
        return;
    };
    let curve_edit_entity = control.curve_edit_entity();
    let canvas_entity = control.canvas_entity();

    let Ok((computed, ui_transform)) = canvases.get(canvas_entity) else {
        return;
    };

    let cursor_pos = event.pointer_location.position / computed.inverse_scale_factor;
    let Some(normalized) = computed.normalize_point(*ui_transform, cursor_pos) else {
        return;
    };

    let Ok(mut state) = states.get_mut(curve_edit_entity) else {
        return;
    };

    let delta = event.delta / computed.inverse_scale_factor;
    control.update_state(&mut state, normalized, Some(delta));

    commands.trigger(CurveEditChangeEvent {
        entity: curve_edit_entity,
        curve: state.curve.clone(),
    });
}

fn on_control_drag_end<C: CurveControl>(
    event: On<Pointer<DragEnd>>,
    mut commands: Commands,
    controls: Query<&C>,
    states: Query<&CurveEditState>,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Ok(control) = controls.get(event.event_target()) else {
        return;
    };
    let curve_edit_entity = control.curve_edit_entity();

    commands.entity(event.event_target()).remove::<Dragging>();

    if let Ok(state) = states.get(curve_edit_entity) {
        commands.trigger(CurveEditCommitEvent {
            entity: curve_edit_entity,
            curve: state.curve.clone(),
        });
    }
}

fn setup_curve_edit(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut curve_edits: Query<(Entity, &CurveEditState, &mut CurveEditConfig)>,
) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    for (entity, state, mut config) in &mut curve_edits {
        if config.initialized {
            continue;
        }
        config.initialized = true;

        let label_entity = commands
            .spawn((
                Text::new("Curve"),
                TextFont {
                    font: font.clone(),
                    font_size: TEXT_SIZE_SM,
                    weight: FontWeight::MEDIUM,
                    ..default()
                },
                TextColor(TEXT_MUTED_COLOR.into()),
            ))
            .id();
        commands.entity(entity).add_child(label_entity);

        let trigger_entity = commands
            .spawn((
                CurveEditTrigger(entity),
                button(
                    ButtonProps::new(state.label())
                        .align_left()
                        .with_left_icon(ICON_CURVE)
                        .with_right_icon(ICON_MORE),
                ),
            ))
            .id();

        commands.entity(entity).add_child(trigger_entity);
    }
}

fn handle_trigger_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    triggers: Query<&CurveEditTrigger>,
    mut states: Query<&mut CurveEditState>,
    existing_popovers: Query<(Entity, &CurveEditPopover)>,
    all_popovers: Query<Entity, With<EditorPopover>>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
    parents: Query<&ChildOf>,
) {
    let Ok(curve_trigger) = triggers.get(trigger.entity) else {
        return;
    };

    let curve_edit_entity = curve_trigger.0;
    let Ok(mut state) = states.get_mut(curve_edit_entity) else {
        return;
    };

    for (popover_entity, popover_ref) in &existing_popovers {
        if popover_ref.0 == curve_edit_entity {
            commands.entity(popover_entity).try_despawn();
            state.popover = None;
            if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(trigger.entity) {
                *variant = ButtonVariant::Default;
                set_button_variant(ButtonVariant::Default, &mut bg, &mut border);
            }
            return;
        }
    }

    let any_popover_open = !all_popovers.is_empty();
    if any_popover_open {
        let is_nested = all_popovers
            .iter()
            .any(|popover| is_descendant_of(curve_edit_entity, popover, &parents));
        if !is_nested {
            return;
        }
    }

    if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(trigger.entity) {
        *variant = ButtonVariant::ActiveAlt;
        set_button_variant(ButtonVariant::ActiveAlt, &mut bg, &mut border);
    }

    let presets = vec![
        ComboBoxOptionData::new("Constant"),
        ComboBoxOptionData::new("Linear"),
        ComboBoxOptionData::new("Quad in"),
        ComboBoxOptionData::new("Quad out"),
        ComboBoxOptionData::new("Quad in out"),
        ComboBoxOptionData::new("Cubic in"),
        ComboBoxOptionData::new("Cubic out"),
        ComboBoxOptionData::new("Cubic in out"),
        ComboBoxOptionData::new("Quart in"),
        ComboBoxOptionData::new("Quart out"),
        ComboBoxOptionData::new("Quart in out"),
        ComboBoxOptionData::new("Quint in"),
        ComboBoxOptionData::new("Quint out"),
        ComboBoxOptionData::new("Quint in out"),
        ComboBoxOptionData::new("Sine in"),
        ComboBoxOptionData::new("Sine out"),
        ComboBoxOptionData::new("Sine in out"),
        ComboBoxOptionData::new("Expo in"),
        ComboBoxOptionData::new("Expo out"),
        ComboBoxOptionData::new("Expo in out"),
        ComboBoxOptionData::new("Circ in"),
        ComboBoxOptionData::new("Circ out"),
        ComboBoxOptionData::new("Circ in out"),
    ];

    let popover_entity = commands
        .spawn((
            CurveEditPopover(curve_edit_entity),
            popover(
                PopoverProps::new(trigger.entity)
                    .with_placement(PopoverPlacement::Left)
                    .with_padding(0.0)
                    .with_node(Node {
                        width: px(256.0),
                        ..default()
                    }),
            ),
        ))
        .id();

    state.popover = Some(popover_entity);

    commands
        .entity(popover_entity)
        .with_child(popover_header(
            PopoverHeaderProps::new("Curve editor", popover_entity),
            &asset_server,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: percent(100),
                        padding: UiRect::all(px(CONTENT_PADDING)),
                        border: UiRect::bottom(px(1.0)),
                        column_gap: px(8.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(BORDER_COLOR),
                ))
                .with_children(|row| {
                    row.spawn((
                        PresetComboBox(curve_edit_entity),
                        combobox_with_label(presets, "Presets"),
                    ));
                    row.spawn((
                        Node {
                            flex_shrink: 0.0,
                            ..default()
                        },
                    )).with_child((
                        FlipButton(curve_edit_entity),
                        icon_button(
                            IconButtonProps::new(ICON_FLIP).variant(ButtonVariant::Default),
                            &asset_server,
                        ),
                    ));
                });

            parent.spawn((
                CurveEditContent(curve_edit_entity),
                popover_content(),
            ));
        });
}

fn setup_curve_edit_content(
    mut commands: Commands,
    mut curve_materials: ResMut<Assets<CurveMaterial>>,
    states: Query<&CurveEditState>,
    contents: Query<(Entity, &CurveEditContent), Added<CurveEditContent>>,
) {
    for (content_entity, content) in &contents {
        let curve_edit_entity = content.0;
        let Ok(state) = states.get(curve_edit_entity) else {
            continue;
        };

        commands.entity(content_entity).with_children(|parent| {
            let canvas_entity = parent
                .spawn((
                    CurveCanvas {
                        curve_edit: curve_edit_entity,
                        point_count: state.curve.points.len(),
                    },
                    Hovered::default(),
                    Node {
                        width: percent(100.0),
                        aspect_ratio: Some(1.0),
                        ..default()
                    },
                ))
                .id();

            parent.commands().entity(canvas_entity).with_children(|canvas_parent| {
                canvas_parent.spawn((
                    CurveMaterialNode(curve_edit_entity),
                    Pickable::IGNORE,
                    MaterialNode(curve_materials.add(CurveMaterial::from_curve(&state.curve))),
                    Node {
                        position_type: PositionType::Absolute,
                        width: percent(100.0),
                        height: percent(100.0),
                        ..default()
                    },
                ));

                spawn_point_handles(canvas_parent, curve_edit_entity, canvas_entity, &state.curve);
                spawn_tension_handles(canvas_parent, curve_edit_entity, canvas_entity, &state.curve);
            });

            parent.spawn((
                RangeEdit(curve_edit_entity),
                vector_edit(
                    VectorEditProps::default()
                        .with_label("Range")
                        .with_size(VectorSize::Vec2)
                        .with_suffixes(VectorSuffixes::Range)
                        .with_default_values(vec![state.curve.range.min, state.curve.range.max]),
                ),
            ));
        });
    }
}

fn spawn_point_handles(parent: &mut ChildSpawnerCommands, curve_edit_entity: Entity, canvas_entity: Entity, curve: &CurveTexture) {
    let range_span = curve.range.span();

    for (i, point) in curve.points.iter().enumerate() {
        let x = point.position;
        let normalized_value = (point.value as f32 - curve.range.min) / range_span;
        let y = 1.0 - normalized_value;

        parent
            .spawn((
                PointHandle {
                    curve_edit: curve_edit_entity,
                    canvas: canvas_entity,
                    index: i,
                },
                point_handle_style(x, y),
            ))
            .observe(on_control_press::<PointHandle>)
            .observe(on_control_release::<PointHandle>)
            .observe(on_control_drag_start::<PointHandle>)
            .observe(on_control_drag::<PointHandle>)
            .observe(on_control_drag_end::<PointHandle>);
    }
}

fn spawn_tension_handles(parent: &mut ChildSpawnerCommands, curve_edit_entity: Entity, canvas_entity: Entity, curve: &CurveTexture) {
    let range_span = curve.range.span();

    for i in 1..curve.points.len() {
        let p0 = &curve.points[i - 1];
        let p1 = &curve.points[i];

        if p1.mode == CurveMode::Hold {
            continue;
        }

        let mid_x = (p0.position + p1.position) / 2.0;
        let curve_value_at_mid = curve.sample(mid_x);
        let normalized_curve_value = (curve_value_at_mid - curve.range.min) / range_span;
        let y = 1.0 - normalized_curve_value;

        parent
            .spawn((
                TensionHandle {
                    curve_edit: curve_edit_entity,
                    canvas: canvas_entity,
                    index: i,
                },
                tension_handle_style(mid_x, y),
            ))
            .observe(on_control_press::<TensionHandle>)
            .observe(on_control_release::<TensionHandle>)
            .observe(on_control_drag_start::<TensionHandle>)
            .observe(on_control_drag::<TensionHandle>)
            .observe(on_control_drag_end::<TensionHandle>);
    }
}

fn point_handle_style(x: f32, y: f32) -> impl Bundle {
    (
        Pickable::default(),
        Hovered::default(),
        Interaction::None,
        Node {
            position_type: PositionType::Absolute,
            width: px(POINT_HANDLE_SIZE),
            height: px(POINT_HANDLE_SIZE),
            left: percent(x * 100.0 - POINT_HANDLE_SIZE / CANVAS_SIZE * 50.0),
            top: percent(y * 100.0 - POINT_HANDLE_SIZE / CANVAS_SIZE * 50.0),
            border: UiRect::all(px(HANDLE_BORDER)),
            border_radius: BorderRadius::all(px(POINT_HANDLE_SIZE / 2.0)),
            ..default()
        },
        BackgroundColor(BACKGROUND_COLOR.into()),
        BorderColor::all(PRIMARY_COLOR),
    )
}

fn tension_handle_style(x: f32, y: f32) -> impl Bundle {
    (
        Pickable::default(),
        Hovered::default(),
        Interaction::None,
        Node {
            position_type: PositionType::Absolute,
            width: px(TENSION_HANDLE_SIZE),
            height: px(TENSION_HANDLE_SIZE),
            left: percent(x * 100.0 - TENSION_HANDLE_SIZE / CANVAS_SIZE * 50.0),
            top: percent(y * 100.0 - TENSION_HANDLE_SIZE / CANVAS_SIZE * 50.0),
            border: UiRect::all(px(HANDLE_BORDER)),
            border_radius: BorderRadius::all(px(TENSION_HANDLE_SIZE / 2.0)),
            ..default()
        },
        BackgroundColor(BACKGROUND_COLOR.into()),
        BorderColor::all(PRIMARY_COLOR),
    )
}

fn update_curve_visuals(
    states: Query<&CurveEditState, Changed<CurveEditState>>,
    material_nodes: Query<(&CurveMaterialNode, &MaterialNode<CurveMaterial>)>,
    mut curve_materials: ResMut<Assets<CurveMaterial>>,
    mut point_handles: Query<(&PointHandle, &mut Node), Without<TensionHandle>>,
    mut tension_handles: Query<(&TensionHandle, &mut Node), Without<PointHandle>>,
) {
    for state in &states {
        let curve_edit_entity = match material_nodes
            .iter()
            .find(|(m, _)| states.get(m.0).is_ok())
        {
            Some((m, _)) => m.0,
            None => continue,
        };

        if states.get(curve_edit_entity).is_err() {
            continue;
        }

        for (mat_node, material_node) in &material_nodes {
            if mat_node.0 != curve_edit_entity {
                continue;
            }
            if let Some(material) = curve_materials.get_mut(&material_node.0) {
                *material = CurveMaterial::from_curve(&state.curve);
            }
        }

        let range_span = state.curve.range.span();

        for (handle, mut node) in &mut point_handles {
            if handle.curve_edit != curve_edit_entity {
                continue;
            }
            let Some(point) = state.curve.points.get(handle.index) else {
                continue;
            };

            let x = point.position;
            let normalized_value = (point.value as f32 - state.curve.range.min) / range_span;
            let y = 1.0 - normalized_value;

            node.left = percent(x * 100.0 - POINT_HANDLE_SIZE / CANVAS_SIZE * 50.0);
            node.top = percent(y * 100.0 - POINT_HANDLE_SIZE / CANVAS_SIZE * 50.0);
        }

        for (handle, mut node) in &mut tension_handles {
            if handle.curve_edit != curve_edit_entity {
                continue;
            }
            if handle.index == 0 || handle.index >= state.curve.points.len() {
                continue;
            }

            let p0 = &state.curve.points[handle.index - 1];
            let p1 = &state.curve.points[handle.index];

            let mid_x = (p0.position + p1.position) / 2.0;
            let curve_value_at_mid = state.curve.sample(mid_x);
            let normalized_curve_value = (curve_value_at_mid - state.curve.range.min) / range_span;
            let y = 1.0 - normalized_curve_value;

            node.left = percent(mid_x * 100.0 - TENSION_HANDLE_SIZE / CANVAS_SIZE * 50.0);
            node.top = percent(y * 100.0 - TENSION_HANDLE_SIZE / CANVAS_SIZE * 50.0);
        }
    }
}

fn respawn_handles_on_point_change(
    mut commands: Commands,
    states: Query<(Entity, &CurveEditState), Changed<CurveEditState>>,
    mut canvases: Query<(Entity, &mut CurveCanvas)>,
    point_handles: Query<(Entity, &PointHandle)>,
    tension_handles: Query<(Entity, &TensionHandle)>,
) {
    for (curve_edit_entity, state) in &states {
        for (canvas_entity, mut canvas) in &mut canvases {
            if canvas.curve_edit != curve_edit_entity {
                continue;
            }

            let current_point_count = state.curve.points.len();
            if canvas.point_count == current_point_count {
                continue;
            }

            canvas.point_count = current_point_count;

            for (handle_entity, handle) in &point_handles {
                if handle.curve_edit == canvas.curve_edit {
                    commands.entity(handle_entity).despawn();
                }
            }

            for (handle_entity, handle) in &tension_handles {
                if handle.curve_edit == canvas.curve_edit {
                    commands.entity(handle_entity).despawn();
                }
            }

            commands.entity(canvas_entity).with_children(|parent| {
                spawn_point_handles(parent, canvas.curve_edit, canvas_entity, &state.curve);
                spawn_tension_handles(parent, canvas.curve_edit, canvas_entity, &state.curve);
            });
        }
    }
}

fn update_handle_cursors(
    mut commands: Commands,
    window: Single<(Entity, Option<&CursorIcon>), With<Window>>,
    point_handles: Query<(&Hovered, Has<Dragging>), With<PointHandle>>,
    tension_handles: Query<(&Hovered, Has<Dragging>), With<TensionHandle>>,
) {
    let (window_entity, current_cursor) = *window;

    let mut new_cursor: Option<SystemCursorIcon> = None;

    for (hovered, is_dragging) in &point_handles {
        if is_dragging {
            new_cursor = Some(SystemCursorIcon::Grabbing);
            break;
        } else if hovered.get() && new_cursor.is_none() {
            new_cursor = Some(SystemCursorIcon::Grab);
        }
    }

    if new_cursor.is_none() {
        for (hovered, is_dragging) in &tension_handles {
            if is_dragging {
                new_cursor = Some(SystemCursorIcon::Grabbing);
                break;
            } else if hovered.get() && new_cursor.is_none() {
                new_cursor = Some(SystemCursorIcon::NsResize);
            }
        }
    }

    if let Some(cursor) = new_cursor {
        commands.entity(window_entity).insert(CursorIcon::from(cursor));
    } else if current_cursor.is_some_and(|c| matches!(c, CursorIcon::System(SystemCursorIcon::Grab | SystemCursorIcon::Grabbing | SystemCursorIcon::NsResize))) {
        commands.entity(window_entity).remove::<CursorIcon>();
    }
}

fn update_handle_colors(
    mut removed_dragging: RemovedComponents<Dragging>,
    mut handles: ParamSet<(
        Query<
            (Entity, &Hovered, Has<Dragging>, &mut BackgroundColor),
            (Or<(With<PointHandle>, With<TensionHandle>)>, Or<(Changed<Hovered>, Added<Dragging>)>),
        >,
        Query<
            (&Hovered, &mut BackgroundColor),
            Or<(With<PointHandle>, With<TensionHandle>)>,
        >,
    )>,
) {
    let removed: Vec<Entity> = removed_dragging.read().collect();
    let hover_color = BACKGROUND_COLOR.mix(&PRIMARY_COLOR, 0.8);

    for (entity, hovered, is_dragging, mut bg) in &mut handles.p0() {
        if removed.contains(&entity) {
            continue;
        }
        *bg = if is_dragging {
            BackgroundColor(PRIMARY_COLOR.into())
        } else if hovered.get() {
            BackgroundColor(hover_color.into())
        } else {
            BackgroundColor(BACKGROUND_COLOR.into())
        };
    }

    for entity in removed {
        if let Ok((hovered, mut bg)) = handles.p1().get_mut(entity) {
            *bg = if hovered.get() {
                BackgroundColor(hover_color.into())
            } else {
                BackgroundColor(BACKGROUND_COLOR.into())
            };
        }
    }
}

// tension values for power-based easings: tension = (1 - 1/exp) / 0.999
const QUAD_TENSION: f64 = 0.5005005005;
const CUBIC_TENSION: f64 = 0.6673340007;
const QUART_TENSION: f64 = 0.7507507508;
const QUINT_TENSION: f64 = 0.8008008008;

struct CurvePresetDef {
    name: &'static str,
    mode: CurveMode,
    easing: CurveEasing,
    tension: f64,
}

impl CurvePresetDef {
    const fn new(name: &'static str, mode: CurveMode, easing: CurveEasing, tension: f64) -> Self {
        Self { name, mode, easing, tension }
    }

    fn to_curve(&self, range: aracari::prelude::ParticleRange) -> CurveTexture {
        CurveTexture::new(vec![
            CurvePoint::new(0.0, 0.0),
            CurvePoint::new(1.0, 1.0)
                .with_mode(self.mode)
                .with_easing(self.easing)
                .with_tension(self.tension),
        ])
        .with_range(range)
    }
}

const CURVE_PRESETS: &[CurvePresetDef] = &[
    // Quad
    CurvePresetDef::new("Quad in", CurveMode::SingleCurve, CurveEasing::Power, QUAD_TENSION),
    CurvePresetDef::new("Quad out", CurveMode::SingleCurve, CurveEasing::Power, -QUAD_TENSION),
    CurvePresetDef::new("Quad in out", CurveMode::DoubleCurve, CurveEasing::Power, QUAD_TENSION),
    // Cubic
    CurvePresetDef::new("Cubic in", CurveMode::SingleCurve, CurveEasing::Power, CUBIC_TENSION),
    CurvePresetDef::new("Cubic out", CurveMode::SingleCurve, CurveEasing::Power, -CUBIC_TENSION),
    CurvePresetDef::new("Cubic in out", CurveMode::DoubleCurve, CurveEasing::Power, CUBIC_TENSION),
    // Quart
    CurvePresetDef::new("Quart in", CurveMode::SingleCurve, CurveEasing::Power, QUART_TENSION),
    CurvePresetDef::new("Quart out", CurveMode::SingleCurve, CurveEasing::Power, -QUART_TENSION),
    CurvePresetDef::new("Quart in out", CurveMode::DoubleCurve, CurveEasing::Power, QUART_TENSION),
    // Quint
    CurvePresetDef::new("Quint in", CurveMode::SingleCurve, CurveEasing::Power, QUINT_TENSION),
    CurvePresetDef::new("Quint out", CurveMode::SingleCurve, CurveEasing::Power, -QUINT_TENSION),
    CurvePresetDef::new("Quint in out", CurveMode::DoubleCurve, CurveEasing::Power, QUINT_TENSION),
    // Sine
    CurvePresetDef::new("Sine in", CurveMode::SingleCurve, CurveEasing::Sine, 1.0),
    CurvePresetDef::new("Sine out", CurveMode::SingleCurve, CurveEasing::Sine, -1.0),
    CurvePresetDef::new("Sine in out", CurveMode::DoubleCurve, CurveEasing::Sine, 1.0),
    // Expo
    CurvePresetDef::new("Expo in", CurveMode::SingleCurve, CurveEasing::Expo, 1.0),
    CurvePresetDef::new("Expo out", CurveMode::SingleCurve, CurveEasing::Expo, -1.0),
    CurvePresetDef::new("Expo in out", CurveMode::DoubleCurve, CurveEasing::Expo, 1.0),
    // Circ
    CurvePresetDef::new("Circ in", CurveMode::SingleCurve, CurveEasing::Circ, 1.0),
    CurvePresetDef::new("Circ out", CurveMode::SingleCurve, CurveEasing::Circ, -1.0),
    CurvePresetDef::new("Circ in out", CurveMode::DoubleCurve, CurveEasing::Circ, 1.0),
];

fn handle_preset_change(
    trigger: On<ComboBoxChangeEvent>,
    mut commands: Commands,
    preset_boxes: Query<&PresetComboBox>,
    mut states: Query<&mut CurveEditState>,
) {
    let Ok(preset_box) = preset_boxes.get(trigger.entity) else {
        return;
    };

    let curve_edit_entity = preset_box.0;
    let Ok(mut state) = states.get_mut(curve_edit_entity) else {
        return;
    };

    let range = state.curve.range;

    match trigger.selected {
        0 => {
            state.curve = CurveTexture::new(vec![
                CurvePoint::new(0.0, 1.0),
                CurvePoint::new(1.0, 1.0),
            ])
            .with_range(range);
            state.preset_name = Some("Constant".to_string());
        }
        1 => {
            state.curve = CurveTexture::new(vec![
                CurvePoint::new(0.0, 0.0),
                CurvePoint::new(1.0, 1.0),
            ])
            .with_range(range);
            state.preset_name = Some("Linear".to_string());
        }
        n if n >= 2 && n < 2 + CURVE_PRESETS.len() => {
            let preset = &CURVE_PRESETS[n - 2];
            state.curve = preset.to_curve(range);
            state.preset_name = Some(preset.name.to_string());
        }
        _ => {}
    }

    commands.trigger(CurveEditChangeEvent {
        entity: curve_edit_entity,
        curve: state.curve.clone(),
    });
    commands.trigger(CurveEditCommitEvent {
        entity: curve_edit_entity,
        curve: state.curve.clone(),
    });
}

fn handle_flip_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    flip_buttons: Query<&FlipButton>,
    mut states: Query<&mut CurveEditState>,
) {
    let Ok(flip_button) = flip_buttons.get(trigger.entity) else {
        return;
    };

    let curve_edit_entity = flip_button.0;
    let Ok(mut state) = states.get_mut(curve_edit_entity) else {
        return;
    };

    // collect interpolation properties from points (skip first, it doesn't control any segment)
    let interp_props: Vec<_> = state.curve.points.iter()
        .skip(1)
        .map(|p| (p.mode, p.easing, p.tension))
        .collect();

    // flip positions
    for point in &mut state.curve.points {
        point.position = 1.0 - point.position;
    }

    // reverse points array
    state.curve.points.reverse();

    // reset first point's interpolation properties (it doesn't control any segment)
    if let Some(first) = state.curve.points.first_mut() {
        first.mode = CurveMode::default();
        first.easing = CurveEasing::default();
        first.tension = 0.0;
    }

    // apply reversed interpolation properties to subsequent points
    for (i, (mode, easing, tension)) in interp_props.iter().rev().enumerate() {
        if let Some(point) = state.curve.points.get_mut(i + 1) {
            point.mode = *mode;
            point.easing = *easing;
            point.tension = *tension;
        }
    }

    state.mark_custom();

    commands.trigger(CurveEditChangeEvent {
        entity: curve_edit_entity,
        curve: state.curve.clone(),
    });
    commands.trigger(CurveEditCommitEvent {
        entity: curve_edit_entity,
        curve: state.curve.clone(),
    });
}

fn sync_trigger_label(
    states: Query<&CurveEditState>,
    changed_states: Query<Entity, Changed<CurveEditState>>,
    triggers: Query<(Entity, &CurveEditTrigger, &Children)>,
    new_trigger_children: Query<Entity, (With<CurveEditTrigger>, Added<Children>)>,
    mut texts: Query<&mut Text>,
) {
    // sync when state changes
    for curve_edit_entity in &changed_states {
        let Ok(state) = states.get(curve_edit_entity) else {
            continue;
        };
        for (_, trigger, children) in &triggers {
            if trigger.0 != curve_edit_entity {
                continue;
            }
            for child in children.iter() {
                if let Ok(mut text) = texts.get_mut(child) {
                    **text = state.label().to_string();
                    break;
                }
            }
        }
    }

    // sync when trigger children are first available (button setup just ran)
    for trigger_entity in &new_trigger_children {
        let Ok((_, trigger, children)) = triggers.get(trigger_entity) else {
            continue;
        };
        let curve_edit_entity = trigger.0;
        if changed_states.get(curve_edit_entity).is_ok() {
            continue; // already handled above
        }
        let Ok(state) = states.get(curve_edit_entity) else {
            continue;
        };
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                **text = state.label().to_string();
                break;
            }
        }
    }
}

fn sync_range_inputs_to_state(
    input_focus: Res<InputFocus>,
    states: Query<(Entity, &CurveEditState), Changed<CurveEditState>>,
    range_edits: Query<(Entity, &RangeEdit, &Children)>,
    vector_edits: Query<&Children, With<EditorVectorEdit>>,
    mut text_inputs: Query<(Entity, &mut TextInputQueue, &ChildOf), With<EditorTextEdit>>,
    parents: Query<&ChildOf>,
) {
    for (curve_edit_entity, state) in &states {
        for (_range_edit_entity, range_edit, range_children) in &range_edits {
            if range_edit.0 != curve_edit_entity {
                continue;
            }

            let values = [state.curve.range.min, state.curve.range.max];

            for range_child in range_children.iter() {
                let Ok(vector_children) = vector_edits.get(range_child) else {
                    continue;
                };

                for (i, vector_child) in vector_children.iter().enumerate() {
                    let Some(&value) = values.get(i) else {
                        continue;
                    };
                    let text = value.to_string();

                    for (text_input_entity, mut queue, child_of) in &mut text_inputs {
                        if input_focus.0 == Some(text_input_entity) {
                            continue;
                        }

                        let mut current = child_of.parent();
                        let mut is_descendant = false;

                        for _ in 0..10 {
                            if current == vector_child {
                                is_descendant = true;
                                break;
                            }
                            if let Ok(parent) = parents.get(current) {
                                current = parent.parent();
                            } else {
                                break;
                            }
                        }

                        if is_descendant {
                            queue.add(TextInputAction::Edit(TextInputEdit::SelectAll));
                            queue.add(TextInputAction::Edit(TextInputEdit::Paste(text.clone())));
                        }
                    }
                }
            }
        }
    }
}

fn handle_popover_closed(
    mut states: Query<(Entity, &mut CurveEditState), With<EditorCurveEdit>>,
    triggers: Query<(Entity, &CurveEditTrigger)>,
    popovers: Query<Entity, With<EditorPopover>>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
) {
    for (_curve_edit_entity, mut state) in &mut states {
        let Some(popover_entity) = state.popover else {
            continue;
        };

        if popovers.get(popover_entity).is_ok() {
            continue;
        }

        state.popover = None;

        for (trigger_entity, trigger) in &triggers {
            if trigger.0 != _curve_edit_entity {
                continue;
            }
            if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(trigger_entity) {
                *variant = ButtonVariant::Default;
                set_button_variant(ButtonVariant::Default, &mut bg, &mut border);
            }
        }
    }
}

fn handle_range_blur(
    input_focus: Res<InputFocus>,
    mut last_focus: Local<Option<Entity>>,
    mut commands: Commands,
    mut states: Query<&mut CurveEditState>,
    range_edits: Query<(Entity, &RangeEdit, &Children)>,
    vector_edits: Query<&Children, With<EditorVectorEdit>>,
    text_inputs: Query<&bevy_ui_text_input::TextInputBuffer, With<EditorTextEdit>>,
    parents: Query<&ChildOf>,
) {
    let current_focus = input_focus.0;
    let previous_focus = *last_focus;
    *last_focus = current_focus;

    let Some(blurred_entity) = previous_focus else {
        return;
    };
    if current_focus == Some(blurred_entity) {
        return;
    }

    let Ok(buffer) = text_inputs.get(blurred_entity) else {
        return;
    };

    for (range_edit_entity, range_edit, range_children) in &range_edits {
        let Ok(mut state) = states.get_mut(range_edit.0) else {
            continue;
        };

        for range_child in range_children.iter() {
            let Ok(vector_children) = vector_edits.get(range_child) else {
                continue;
            };

            for (field_index, vector_child) in vector_children.iter().enumerate() {
                let is_descendant = is_descendant_of(blurred_entity, vector_child, &parents);
                if !is_descendant {
                    continue;
                }

                let text = buffer.get_text();
                if text.is_empty() {
                    return;
                }

                let Ok(value) = text.parse::<f32>() else {
                    return;
                };

                let mut changed = false;
                if field_index == 0 {
                    if (state.curve.range.min - value).abs() > f32::EPSILON {
                        state.curve.range.min = value;
                        changed = true;
                    }
                } else if (state.curve.range.max - value).abs() > f32::EPSILON {
                    state.curve.range.max = value;
                    changed = true;
                }

                if changed {
                    let range_min = state.curve.range.min as f64;
                    let range_max = state.curve.range.max as f64;
                    for point in &mut state.curve.points {
                        point.value = point.value.clamp(range_min, range_max);
                    }

                    state.mark_custom();
                    commands.trigger(CurveEditChangeEvent {
                        entity: range_edit.0,
                        curve: state.curve.clone(),
                    });
                    commands.trigger(CurveEditCommitEvent {
                        entity: range_edit.0,
                        curve: state.curve.clone(),
                    });
                }

                return;
            }
        }
    }
}

fn handle_canvas_right_click(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    canvases: Query<(&CurveCanvas, &ComputedNode, &UiGlobalTransform, &Hovered)>,
    mut states: Query<&mut CurveEditState>,
    point_handles: Query<&Hovered, With<PointHandle>>,
    tension_handles: Query<&Hovered, With<TensionHandle>>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }

    let point_hovered = point_handles.iter().any(|h| h.get());
    let tension_hovered = tension_handles.iter().any(|h| h.get());
    if point_hovered || tension_hovered {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    for (canvas, computed, ui_transform, hovered) in &canvases {
        if !hovered.get() {
            continue;
        }

        let Ok(mut state) = states.get_mut(canvas.curve_edit) else {
            continue;
        };

        if state.curve.points.len() >= MAX_POINTS {
            continue;
        }

        let cursor_pos = cursor_position / computed.inverse_scale_factor;
        let Some(normalized) = computed.normalize_point(*ui_transform, cursor_pos) else {
            continue;
        };

        let normalized_x = (normalized.x + 0.5).clamp(0.0, 1.0);
        let normalized_y = (0.5 - normalized.y).clamp(0.0, 1.0);

        let range_min = state.curve.range.min as f64;
        let range_span = state.curve.range.span() as f64;
        let value = range_min + normalized_y as f64 * range_span;

        let new_point = CurvePoint::new(normalized_x, value)
            .with_mode(CurveMode::DoubleCurve)
            .with_tension(0.0);

        let insert_idx = state
            .curve
            .points
            .iter()
            .position(|p| p.position > normalized_x)
            .unwrap_or(state.curve.points.len());

        state.curve.points.insert(insert_idx, new_point);
        state.mark_custom();

        commands.trigger(CurveEditChangeEvent {
            entity: canvas.curve_edit,
            curve: state.curve.clone(),
        });
        commands.trigger(CurveEditCommitEvent {
            entity: canvas.curve_edit,
            curve: state.curve.clone(),
        });

        break;
    }
}

fn menu_separator() -> impl Bundle {
    (
        Node {
            width: percent(100.0),
            height: px(1.0),
            margin: UiRect::vertical(px(4.0)),
            ..default()
        },
        BackgroundColor(BORDER_COLOR.into()),
    )
}

fn menu_button_variant(is_active: bool, is_disabled: bool) -> ButtonVariant {
    if is_disabled {
        ButtonVariant::Disabled
    } else if is_active {
        ButtonVariant::Active
    } else {
        ButtonVariant::Ghost
    }
}

fn spawn_enum_options<T, C, F>(
    parent: &mut ChildSpawnerCommands,
    current: T,
    is_disabled: bool,
    make_component: F,
)
where
    T: Typed + PartialEq + std::str::FromStr + Copy,
    C: Component,
    F: Fn(T, bool) -> C,
{
    let bevy::reflect::TypeInfo::Enum(info) = T::type_info() else {
        return;
    };

    for variant_info in info.iter() {
        let Ok(value) = variant_info.name().parse::<T>() else {
            continue;
        };
        let name = variant_info.name().to_sentence_case();
        let is_active = value == current && !is_disabled;
        let variant = menu_button_variant(is_active, is_disabled);

        parent.spawn((
            make_component(value, is_disabled),
            button(ButtonProps::new(&name).with_variant(variant).align_left()),
        ));
    }
}

fn spawn_mode_options(
    parent: &mut ChildSpawnerCommands,
    curve_edit: Entity,
    point_index: usize,
    current_mode: CurveMode,
    is_first: bool,
) {
    spawn_enum_options(parent, current_mode, is_first, |mode, disabled| {
        ModeOption { curve_edit, point_index, mode, disabled }
    });
}

fn spawn_easing_options(
    parent: &mut ChildSpawnerCommands,
    curve_edit: Entity,
    point_index: usize,
    current_easing: CurveEasing,
    is_first: bool,
) {
    spawn_enum_options(parent, current_easing, is_first, |easing, disabled| {
        EasingOption { curve_edit, point_index, easing, disabled }
    });
}

fn spawn_delete_option(
    parent: &mut ChildSpawnerCommands,
    curve_edit: Entity,
    point_index: usize,
    can_delete: bool,
) {
    let variant = menu_button_variant(false, !can_delete);

    parent.spawn((
        DeletePointOption {
            curve_edit,
            point_index,
            disabled: !can_delete,
        },
        button(ButtonProps::new("Delete").with_variant(variant).align_left()),
    ));
}

fn handle_point_right_click(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    point_handles: Query<(Entity, &PointHandle, &Hovered)>,
    states: Query<&CurveEditState>,
    existing_menus: Query<Entity, With<PointModeMenu>>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }

    for menu_entity in &existing_menus {
        commands.entity(menu_entity).try_despawn();
    }

    for (handle_entity, point_handle, hovered) in &point_handles {
        if !hovered.get() {
            continue;
        }

        let Ok(state) = states.get(point_handle.curve_edit) else {
            continue;
        };

        let Some(point) = state.curve.points.get(point_handle.index) else {
            continue;
        };

        let is_first = point_handle.index == 0;
        let can_delete = state.curve.points.len() > 2;

        let popover_entity = commands
            .spawn((
                PointModeMenu {
                    curve_edit: point_handle.curve_edit,
                    index: point_handle.index,
                },
                popover(
                    PopoverProps::new(handle_entity)
                        .with_placement(PopoverPlacement::BottomStart)
                        .with_padding(4.0)
                        .with_z_index(300),
                ),
            ))
            .id();

        commands.entity(popover_entity).with_children(|parent| {
            spawn_mode_options(parent, point_handle.curve_edit, point_handle.index, point.mode, is_first);
            parent.spawn(menu_separator());
            spawn_easing_options(parent, point_handle.curve_edit, point_handle.index, point.easing, is_first);
            parent.spawn(menu_separator());
            spawn_delete_option(parent, point_handle.curve_edit, point_handle.index, can_delete);
        });

        break;
    }
}

#[derive(Component)]
struct ModeOption {
    curve_edit: Entity,
    point_index: usize,
    mode: CurveMode,
    disabled: bool,
}

#[derive(Component)]
struct DeletePointOption {
    curve_edit: Entity,
    point_index: usize,
    disabled: bool,
}

#[derive(Component)]
struct EasingOption {
    curve_edit: Entity,
    point_index: usize,
    easing: CurveEasing,
    disabled: bool,
}

fn handle_point_mode_change(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    mode_options: Query<&ModeOption>,
    easing_options: Query<&EasingOption>,
    delete_options: Query<&DeletePointOption>,
    mut states: Query<&mut CurveEditState>,
    menus: Query<Entity, With<PointModeMenu>>,
) {
    if let Ok(mode_opt) = mode_options.get(trigger.entity) {
        if mode_opt.disabled {
            return;
        }

        let Ok(mut state) = states.get_mut(mode_opt.curve_edit) else {
            return;
        };

        if let Some(point) = state.curve.points.get_mut(mode_opt.point_index) {
            point.mode = mode_opt.mode;
            state.mark_custom();

            commands.trigger(CurveEditChangeEvent {
                entity: mode_opt.curve_edit,
                curve: state.curve.clone(),
            });
            commands.trigger(CurveEditCommitEvent {
                entity: mode_opt.curve_edit,
                curve: state.curve.clone(),
            });
        }

        for menu in &menus {
            commands.entity(menu).try_despawn();
        }

        return;
    }

    if let Ok(easing_opt) = easing_options.get(trigger.entity) {
        if easing_opt.disabled {
            return;
        }

        let Ok(mut state) = states.get_mut(easing_opt.curve_edit) else {
            return;
        };

        if let Some(point) = state.curve.points.get_mut(easing_opt.point_index) {
            point.easing = easing_opt.easing;
            state.mark_custom();

            commands.trigger(CurveEditChangeEvent {
                entity: easing_opt.curve_edit,
                curve: state.curve.clone(),
            });
            commands.trigger(CurveEditCommitEvent {
                entity: easing_opt.curve_edit,
                curve: state.curve.clone(),
            });
        }

        for menu in &menus {
            commands.entity(menu).try_despawn();
        }

        return;
    }

    if let Ok(delete_opt) = delete_options.get(trigger.entity) {
        if delete_opt.disabled {
            return;
        }

        let Ok(mut state) = states.get_mut(delete_opt.curve_edit) else {
            return;
        };

        if state.curve.points.len() <= 2 {
            return;
        }

        state.curve.points.remove(delete_opt.point_index);
        state.mark_custom();

        commands.trigger(CurveEditChangeEvent {
            entity: delete_opt.curve_edit,
            curve: state.curve.clone(),
        });
        commands.trigger(CurveEditCommitEvent {
            entity: delete_opt.curve_edit,
            curve: state.curve.clone(),
        });

        for menu in &menus {
            commands.entity(menu).try_despawn();
        }
    }
}

fn handle_tension_right_click(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    tension_handles: Query<(&TensionHandle, &Hovered)>,
    mut states: Query<&mut CurveEditState>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
        return;
    }

    for (tension_handle, hovered) in &tension_handles {
        if !hovered.get() {
            continue;
        }

        let Ok(mut state) = states.get_mut(tension_handle.curve_edit) else {
            continue;
        };

        if let Some(point) = state.curve.points.get_mut(tension_handle.index) {
            point.tension = 0.0;
            state.mark_custom();

            commands.trigger(CurveEditChangeEvent {
                entity: tension_handle.curve_edit,
                curve: state.curve.clone(),
            });
            commands.trigger(CurveEditCommitEvent {
                entity: tension_handle.curve_edit,
                curve: state.curve.clone(),
            });
        }

        break;
    }
}
