use bevy::input_focus::InputFocus;
use bevy::picking::events::{Press, Release};
use bevy::picking::prelude::Pickable;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::*;
use bevy::shader::ShaderRef;
use bevy::ui::UiGlobalTransform;
use bevy_ui_text_input::TextInputQueue;
use bevy_ui_text_input::actions::{TextInputAction, TextInputEdit};

use crate::ui::widgets::button::{
    ButtonClickEvent, ButtonProps, ButtonVariant, button, set_button_variant,
};

use crate::ui::widgets::combobox::{ComboBoxChangeEvent, combobox_with_selected};
use crate::ui::widgets::popover::{
    EditorPopover, PopoverHeaderProps, PopoverPlacement, PopoverProps, popover, popover_header,
};
use crate::ui::widgets::text_edit::{EditorTextEdit, TextEditPrefix, TextEditProps, text_edit};

const SHADER_HSV_RECT_PATH: &str = "shaders/color_picker_hsv_rect.wgsl";
const SHADER_HUE_PATH: &str = "shaders/color_picker_hue.wgsl";
const SHADER_ALPHA_PATH: &str = "shaders/color_picker_alpha.wgsl";
const SHADER_CHECKERBOARD_PATH: &str = "shaders/color_picker_checkerboard.wgsl";
const CONTENT_WIDTH: f32 = 288.0;
const CONTENT_PADDING: f32 = 12.0;
const SLIDER_HEIGHT: f32 = 18.0;
const HANDLE_SIZE: f32 = 12.0;
const HANDLE_BORDER: f32 = 1.0;
const SWATCH_SIZE: f32 = 16.0;
const CHECKERBOARD_SIZE: f32 = 8.0;
const BORDER_RADIUS: f32 = 4.0;

pub fn plugin(app: &mut App) {
    app.add_plugins(UiMaterialPlugin::<HsvRectMaterial>::default())
        .add_plugins(UiMaterialPlugin::<HueSliderMaterial>::default())
        .add_plugins(UiMaterialPlugin::<AlphaSliderMaterial>::default())
        .add_plugins(UiMaterialPlugin::<CheckerboardMaterial>::default())
        .add_observer(handle_trigger_click)
        .add_observer(handle_input_mode_change)
        .add_systems(
            Update,
            (
                setup_color_picker,
                setup_trigger_swatch,
                setup_color_picker_content,
                update_color_picker_visuals,
                handle_input_field_blur,
                update_trigger_display,
                sync_text_inputs_to_state,
                handle_popover_closed,
            ),
        );
}

#[derive(Component)]
pub struct EditorColorPicker;

#[derive(Component, Clone)]
pub struct ColorPickerState {
    pub hue: f32,
    pub saturation: f32,
    pub brightness: f32,
    pub alpha: f32,
    pub input_mode: ColorInputMode,
    popover: Option<Entity>,
}

impl Default for ColorPickerState {
    fn default() -> Self {
        Self {
            hue: 0.0,
            saturation: 0.0,
            brightness: 1.0,
            alpha: 1.0,
            input_mode: ColorInputMode::Rgb,
            popover: None,
        }
    }
}

impl ColorPickerState {
    pub fn from_rgba(rgba: [f32; 4]) -> Self {
        let (h, s, v) = rgb_to_hsv(rgba[0], rgba[1], rgba[2]);
        Self {
            hue: h,
            saturation: s,
            brightness: v,
            alpha: rgba[3],
            input_mode: ColorInputMode::Rgb,
            popover: None,
        }
    }

    pub fn to_rgba(&self) -> [f32; 4] {
        let (r, g, b) = hsv_to_rgb(self.hue, self.saturation, self.brightness);
        [r, g, b, self.alpha]
    }

    pub fn set_from_rgba(&mut self, rgba: [f32; 4]) {
        let (h, s, v) = rgb_to_hsv(rgba[0], rgba[1], rgba[2]);
        self.hue = h;
        self.saturation = s;
        self.brightness = v;
        self.alpha = rgba[3];
    }

    pub fn to_srgba(&self) -> Srgba {
        let rgba = self.to_rgba();
        Srgba::new(rgba[0], rgba[1], rgba[2], rgba[3])
    }

    pub fn to_hex(&self) -> String {
        let rgba = self.to_rgba();
        let r = (rgba[0] * 255.0).round() as u8;
        let g = (rgba[1] * 255.0).round() as u8;
        let b = (rgba[2] * 255.0).round() as u8;
        format!("{:02X}{:02X}{:02X}", r, g, b)
    }
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum ColorInputMode {
    Hex,
    #[default]
    Rgb,
    Hsb,
}

impl ColorInputMode {
    fn index(&self) -> usize {
        match self {
            Self::Hex => 0,
            Self::Rgb => 1,
            Self::Hsb => 2,
        }
    }

    fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Hex,
            2 => Self::Hsb,
            _ => Self::Rgb,
        }
    }
}

#[derive(EntityEvent)]
pub struct ColorPickerChangeEvent {
    pub entity: Entity,
    #[allow(dead_code)]
    pub color: [f32; 4],
}

#[derive(EntityEvent)]
pub struct ColorPickerCommitEvent {
    pub entity: Entity,
    #[allow(dead_code)]
    pub color: [f32; 4],
}

#[derive(Default)]
pub struct ColorPickerProps {
    pub color: [f32; 4],
    pub inline: bool,
}

impl ColorPickerProps {
    pub fn new() -> Self {
        Self {
            color: [1.0, 1.0, 1.0, 1.0],
            inline: false,
        }
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    #[allow(dead_code)]
    pub fn inline(mut self) -> Self {
        self.inline = true;
        self
    }
}

pub fn color_picker(props: ColorPickerProps) -> impl Bundle {
    let ColorPickerProps { color, inline } = props;

    (
        EditorColorPicker,
        ColorPickerState::from_rgba(color),
        ColorPickerConfig { inline },
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
    )
}

#[derive(Component)]
struct ColorPickerConfig {
    inline: bool,
}

#[derive(Component)]
struct ColorPickerTrigger(Entity);

#[derive(Component)]
struct ColorPickerPopover(Entity);

#[derive(Component)]
struct ColorPickerContent(Entity);

#[derive(Component)]
struct HsvRectangle(Entity);

#[derive(Component)]
struct HsvRectMaterialNode(Entity);

#[derive(Component)]
struct HsvRectHandle(Entity);

#[derive(Component)]
struct HueSlider(Entity);

#[derive(Component)]
struct HueHandle(Entity);

#[derive(Component)]
struct AlphaSlider(Entity);

#[derive(Component)]
struct AlphaMaterialNode(Entity);

#[derive(Component)]
struct AlphaHandle(Entity);

#[derive(Component)]
struct AlphaHandleMaterial(Entity);

#[derive(Component)]
struct ColorInputRow(Entity);

#[derive(Component)]
struct TriggerSwatchConfig {
    picker: Entity,
    color: Srgba,
}

#[derive(Component)]
struct TriggerSwatch;

#[derive(Component)]
pub struct TriggerSwatchMaterial(pub Entity);

#[derive(Component)]
struct TriggerLabel(Entity);

#[derive(Component, Clone, Copy)]
enum InputFieldKind {
    Hex,
    Red,
    Green,
    Blue,
    Hue,
    Saturation,
    Brightness,
    Alpha,
}

impl InputFieldKind {
    fn parse_and_apply(&self, text: &str, state: &mut ColorPickerState) -> bool {
        match self {
            Self::Hex => {
                let Some(rgba) = parse_hex(text) else {
                    return false;
                };
                let (h, s, v) = rgb_to_hsv(rgba[0], rgba[1], rgba[2]);
                state.hue = h;
                state.saturation = s;
                state.brightness = v;
                state.alpha = rgba[3];
                true
            }
            Self::Red | Self::Green | Self::Blue => {
                let Ok(v) = text.parse::<i32>() else {
                    return false;
                };
                let channel = (v.clamp(0, 255) as f32) / 255.0;
                let mut rgba = state.to_rgba();
                match self {
                    Self::Red => rgba[0] = channel,
                    Self::Green => rgba[1] = channel,
                    Self::Blue => rgba[2] = channel,
                    _ => unreachable!(),
                }
                let (h, s, br) = rgb_to_hsv(rgba[0], rgba[1], rgba[2]);
                state.hue = h;
                state.saturation = s;
                state.brightness = br;
                true
            }
            Self::Hue => {
                let Ok(v) = text.parse::<i32>() else {
                    return false;
                };
                state.hue = v.clamp(0, 360) as f32;
                true
            }
            Self::Saturation | Self::Brightness | Self::Alpha => {
                let Ok(v) = text.parse::<i32>() else {
                    return false;
                };
                let value = (v.clamp(0, 100) as f32) / 100.0;
                match self {
                    Self::Saturation => state.saturation = value,
                    Self::Brightness => state.brightness = value,
                    Self::Alpha => state.alpha = value,
                    _ => unreachable!(),
                }
                true
            }
        }
    }

    fn format_value(&self, state: &ColorPickerState) -> String {
        match self {
            Self::Hex => state.to_hex(),
            Self::Red | Self::Green | Self::Blue => {
                let rgba = state.to_rgba();
                let index = match self {
                    Self::Red => 0,
                    Self::Green => 1,
                    Self::Blue => 2,
                    _ => unreachable!(),
                };
                ((rgba[index] * 255.0).round() as i32).to_string()
            }
            Self::Hue => (state.hue.round() as i32).to_string(),
            Self::Saturation => ((state.saturation * 100.0).round() as i32).to_string(),
            Self::Brightness => ((state.brightness * 100.0).round() as i32).to_string(),
            Self::Alpha => ((state.alpha * 100.0).round() as i32).to_string(),
        }
    }
}

#[derive(Component)]
struct ColorInputField {
    picker: Entity,
    kind: InputFieldKind,
}

#[derive(Component, Default)]
struct Dragging;

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct HsvRectMaterial {
    #[uniform(0)]
    pub hue: f32,
    #[uniform(0)]
    pub border_radius: f32,
}

impl UiMaterial for HsvRectMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_HSV_RECT_PATH.into()
    }
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct HueSliderMaterial {
    #[uniform(0)]
    pub border_radius: f32,
}

impl UiMaterial for HueSliderMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_HUE_PATH.into()
    }
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct AlphaSliderMaterial {
    #[uniform(0)]
    pub color: Vec4,
    #[uniform(0)]
    pub checkerboard_size: f32,
    #[uniform(0)]
    pub border_radius: f32,
}

impl UiMaterial for AlphaSliderMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ALPHA_PATH.into()
    }
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct CheckerboardMaterial {
    #[uniform(0)]
    pub color: Vec4,
    #[uniform(0)]
    pub size: f32,
    #[uniform(0)]
    pub border_radius: f32,
}

impl UiMaterial for CheckerboardMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_CHECKERBOARD_PATH.into()
    }
}

trait PickerControl: Component {
    fn picker_entity(&self) -> Entity;
    fn update_state(&self, state: &mut ColorPickerState, normalized: Vec2);
}

impl PickerControl for HsvRectangle {
    fn picker_entity(&self) -> Entity {
        self.0
    }

    fn update_state(&self, state: &mut ColorPickerState, normalized: Vec2) {
        state.saturation = (normalized.x + 0.5).clamp(0.0, 1.0);
        state.brightness = (0.5 - normalized.y).clamp(0.0, 1.0);
    }
}

impl PickerControl for HueSlider {
    fn picker_entity(&self) -> Entity {
        self.0
    }

    fn update_state(&self, state: &mut ColorPickerState, normalized: Vec2) {
        state.hue = (normalized.x + 0.5).clamp(0.0, 1.0) * 360.0;
    }
}

impl PickerControl for AlphaSlider {
    fn picker_entity(&self) -> Entity {
        self.0
    }

    fn update_state(&self, state: &mut ColorPickerState, normalized: Vec2) {
        state.alpha = (normalized.x + 0.5).clamp(0.0, 1.0);
    }
}

fn on_control_press<C: PickerControl>(
    event: On<Pointer<Press>>,
    mut commands: Commands,
    controls: Query<(&C, &ComputedNode, &UiGlobalTransform)>,
    mut pickers: Query<&mut ColorPickerState>,
) {
    let Ok((control, computed, ui_transform)) = controls.get(event.event_target()) else {
        return;
    };
    let picker_entity = control.picker_entity();

    let cursor_pos = event.pointer_location.position / computed.inverse_scale_factor;
    let Some(normalized) = computed.normalize_point(*ui_transform, cursor_pos) else {
        return;
    };

    let Ok(mut state) = pickers.get_mut(picker_entity) else {
        return;
    };

    control.update_state(&mut state, normalized);

    commands.trigger(ColorPickerChangeEvent {
        entity: picker_entity,
        color: state.to_rgba(),
    });
}

fn on_control_release<C: PickerControl>(
    event: On<Pointer<Release>>,
    mut commands: Commands,
    controls: Query<&C, Without<Dragging>>,
    pickers: Query<&ColorPickerState>,
) {
    let Ok(control) = controls.get(event.event_target()) else {
        return;
    };
    let picker_entity = control.picker_entity();

    if let Ok(state) = pickers.get(picker_entity) {
        commands.trigger(ColorPickerCommitEvent {
            entity: picker_entity,
            color: state.to_rgba(),
        });
    }
}

fn on_control_drag_start<C: PickerControl>(
    event: On<Pointer<DragStart>>,
    mut commands: Commands,
    controls: Query<(&C, &ComputedNode, &UiGlobalTransform)>,
    mut pickers: Query<&mut ColorPickerState>,
) {
    let Ok((control, computed, ui_transform)) = controls.get(event.event_target()) else {
        return;
    };
    let picker_entity = control.picker_entity();

    commands.entity(event.event_target()).insert(Dragging);

    let cursor_pos = event.pointer_location.position / computed.inverse_scale_factor;
    let Some(normalized) = computed.normalize_point(*ui_transform, cursor_pos) else {
        return;
    };

    let Ok(mut state) = pickers.get_mut(picker_entity) else {
        return;
    };

    control.update_state(&mut state, normalized);

    commands.trigger(ColorPickerChangeEvent {
        entity: picker_entity,
        color: state.to_rgba(),
    });
}

fn on_control_drag<C: PickerControl>(
    event: On<Pointer<Drag>>,
    mut commands: Commands,
    controls: Query<(&C, &ComputedNode, &UiGlobalTransform), With<Dragging>>,
    mut pickers: Query<&mut ColorPickerState>,
) {
    let Ok((control, computed, ui_transform)) = controls.get(event.event_target()) else {
        return;
    };
    let picker_entity = control.picker_entity();

    let cursor_pos = event.pointer_location.position / computed.inverse_scale_factor;
    let Some(normalized) = computed.normalize_point(*ui_transform, cursor_pos) else {
        return;
    };

    let Ok(mut state) = pickers.get_mut(picker_entity) else {
        return;
    };

    control.update_state(&mut state, normalized);

    commands.trigger(ColorPickerChangeEvent {
        entity: picker_entity,
        color: state.to_rgba(),
    });
}

fn on_control_drag_end<C: PickerControl>(
    event: On<Pointer<DragEnd>>,
    mut commands: Commands,
    controls: Query<&C>,
    pickers: Query<&ColorPickerState>,
) {
    let Ok(control) = controls.get(event.event_target()) else {
        return;
    };
    let picker_entity = control.picker_entity();

    commands.entity(event.event_target()).remove::<Dragging>();

    if let Ok(state) = pickers.get(picker_entity) {
        commands.trigger(ColorPickerCommitEvent {
            entity: picker_entity,
            color: state.to_rgba(),
        });
    }
}

fn setup_color_picker(
    mut commands: Commands,
    mut pickers: Query<(Entity, &ColorPickerConfig, &ColorPickerState), Added<EditorColorPicker>>,
) {
    for (entity, config, state) in &mut pickers {
        if config.inline {
            commands.entity(entity).with_child((
                ColorPickerContent(entity),
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(CONTENT_PADDING),
                    width: px(CONTENT_WIDTH),
                    ..default()
                },
            ));
        } else {
            let rgba = state.to_rgba();
            let srgba = Srgba::new(rgba[0], rgba[1], rgba[2], rgba[3]);
            let hex = state.to_hex();

            let trigger_entity = commands
                .spawn((
                    ColorPickerTrigger(entity),
                    button(
                        ButtonProps::new(hex)
                            .with_variant(ButtonVariant::Default)
                            .align_left(),
                    ),
                ))
                .id();

            commands.entity(entity).add_child(trigger_entity);

            commands.entity(trigger_entity).insert(TriggerSwatchConfig {
                picker: entity,
                color: srgba,
            });
        }
    }
}

fn setup_trigger_swatch(
    mut commands: Commands,
    mut checkerboard_materials: ResMut<Assets<CheckerboardMaterial>>,
    triggers: Query<(Entity, &TriggerSwatchConfig, &Children)>,
    texts: Query<Entity, With<Text>>,
) {
    for (trigger_entity, config, children) in &triggers {
        commands
            .entity(trigger_entity)
            .remove::<TriggerSwatchConfig>();

        let swatch_entity = commands
            .spawn((
                TriggerSwatch,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(6.0),
                    width: px(SWATCH_SIZE),
                    height: px(SWATCH_SIZE),
                    border_radius: BorderRadius::all(px(BORDER_RADIUS)),
                    overflow: Overflow::clip(),
                    ..default()
                },
            ))
            .id();

        commands.entity(swatch_entity).with_children(|parent| {
            parent.spawn((
                TriggerSwatchMaterial(config.picker),
                MaterialNode(checkerboard_materials.add(CheckerboardMaterial {
                    color: Vec4::new(
                        config.color.red,
                        config.color.green,
                        config.color.blue,
                        config.color.alpha,
                    ),
                    size: CHECKERBOARD_SIZE,
                    border_radius: BORDER_RADIUS,
                })),
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
            ));
        });

        commands.entity(trigger_entity).add_child(swatch_entity);

        for child in children.iter() {
            if texts.get(child).is_ok() {
                commands.entity(child).insert((
                    TriggerLabel(config.picker),
                    Node {
                        margin: UiRect::left(px(SWATCH_SIZE + 6.0)),
                        ..default()
                    },
                ));
                break;
            }
        }
    }
}

fn handle_style(left: f32, top: f32, color: Option<Srgba>) -> impl Bundle {
    (
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            width: px(HANDLE_SIZE),
            height: px(HANDLE_SIZE),
            left: px(left),
            top: px(top),
            border: UiRect::all(px(HANDLE_BORDER)),
            border_radius: BorderRadius::all(px(HANDLE_SIZE / 2.0)),
            ..default()
        },
        BackgroundColor(color.unwrap_or(Srgba::NONE).into()),
        BorderColor::all(Srgba::WHITE),
        Outline {
            width: px(1.0),
            color: Srgba::BLACK.into(),
            ..default()
        },
    )
}

fn slider_node() -> Node {
    Node {
        width: percent(100.0),
        height: px(SLIDER_HEIGHT),
        ..default()
    }
}

fn fullsize_absolute_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: percent(100.0),
        height: percent(100.0),
        ..default()
    }
}

fn setup_color_picker_content(
    mut commands: Commands,
    mut hsv_rect_materials: ResMut<Assets<HsvRectMaterial>>,
    mut hue_materials: ResMut<Assets<HueSliderMaterial>>,
    mut alpha_materials: ResMut<Assets<AlphaSliderMaterial>>,
    mut checkerboard_materials: ResMut<Assets<CheckerboardMaterial>>,
    states: Query<&ColorPickerState>,
    contents: Query<(Entity, &ColorPickerContent), Added<ColorPickerContent>>,
) {
    for (content_entity, content) in &contents {
        let picker_entity = content.0;
        let Ok(state) = states.get(picker_entity) else {
            continue;
        };

        commands.entity(content_entity).with_children(|parent| {
            let current_color = state.to_srgba();
            let current_rgb = hsv_to_rgb(state.hue, state.saturation, state.brightness);

            parent
                .spawn((
                    HsvRectangle(picker_entity),
                    Node {
                        width: percent(100.0),
                        aspect_ratio: Some(1.0),
                        ..default()
                    },
                ))
                .with_children(|hsv_rect_parent| {
                    hsv_rect_parent.spawn((
                        HsvRectMaterialNode(picker_entity),
                        Pickable::IGNORE,
                        MaterialNode(hsv_rect_materials.add(HsvRectMaterial {
                            hue: state.hue,
                            border_radius: BORDER_RADIUS,
                        })),
                        fullsize_absolute_node(),
                    ));

                    hsv_rect_parent.spawn((
                        HsvRectHandle(picker_entity),
                        handle_style(
                            state.saturation * CONTENT_WIDTH - HANDLE_SIZE / 2.0,
                            (1.0 - state.brightness) * CONTENT_WIDTH - HANDLE_SIZE / 2.0,
                            Some(current_color.with_alpha(1.0)),
                        ),
                    ));
                })
                .observe(on_control_press::<HsvRectangle>)
                .observe(on_control_release::<HsvRectangle>)
                .observe(on_control_drag_start::<HsvRectangle>)
                .observe(on_control_drag::<HsvRectangle>)
                .observe(on_control_drag_end::<HsvRectangle>);

            parent
                .spawn((HueSlider(picker_entity), slider_node()))
                .with_children(|hue_parent| {
                    hue_parent.spawn((
                        Pickable::IGNORE,
                        MaterialNode(hue_materials.add(HueSliderMaterial {
                            border_radius: BORDER_RADIUS,
                        })),
                        fullsize_absolute_node(),
                    ));

                    let hue_color = hsv_to_rgb(state.hue, 1.0, 1.0);
                    hue_parent.spawn((
                        HueHandle(picker_entity),
                        handle_style(
                            (state.hue / 360.0) * CONTENT_WIDTH - HANDLE_SIZE / 2.0,
                            (SLIDER_HEIGHT - HANDLE_SIZE) / 2.0,
                            Some(Srgba::new(hue_color.0, hue_color.1, hue_color.2, 1.0)),
                        ),
                    ));
                })
                .observe(on_control_press::<HueSlider>)
                .observe(on_control_release::<HueSlider>)
                .observe(on_control_drag_start::<HueSlider>)
                .observe(on_control_drag::<HueSlider>)
                .observe(on_control_drag_end::<HueSlider>);

            parent
                .spawn((AlphaSlider(picker_entity), slider_node()))
                .with_children(|alpha_parent| {
                    alpha_parent.spawn((
                        AlphaMaterialNode(picker_entity),
                        Pickable::IGNORE,
                        MaterialNode(alpha_materials.add(AlphaSliderMaterial {
                            color: Vec4::new(current_rgb.0, current_rgb.1, current_rgb.2, 1.0),
                            checkerboard_size: CHECKERBOARD_SIZE,
                            border_radius: BORDER_RADIUS,
                        })),
                        fullsize_absolute_node(),
                    ));

                    let inner_size = HANDLE_SIZE - HANDLE_BORDER * 2.0;
                    let inner_radius = inner_size / 2.0;
                    alpha_parent
                        .spawn((
                            AlphaHandle(picker_entity),
                            handle_style(
                                state.alpha * CONTENT_WIDTH - HANDLE_SIZE / 2.0,
                                (SLIDER_HEIGHT - HANDLE_SIZE) / 2.0,
                                None,
                            ),
                        ))
                        .with_children(|handle| {
                            handle
                                .spawn((
                                    Pickable::IGNORE,
                                    Node {
                                        width: px(inner_size),
                                        height: px(inner_size),
                                        border_radius: BorderRadius::all(px(inner_radius)),
                                        overflow: Overflow::clip(),
                                        ..default()
                                    },
                                ))
                                .with_children(|swatch| {
                                    swatch.spawn((
                                        AlphaHandleMaterial(picker_entity),
                                        Pickable::IGNORE,
                                        MaterialNode(checkerboard_materials.add(
                                            CheckerboardMaterial {
                                                color: Vec4::new(
                                                    current_color.red,
                                                    current_color.green,
                                                    current_color.blue,
                                                    current_color.alpha,
                                                ),
                                                size: CHECKERBOARD_SIZE,
                                                border_radius: inner_size,
                                            },
                                        )),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            width: percent(100.0),
                                            height: percent(100.0),
                                            ..default()
                                        },
                                    ));
                                });
                        });
                })
                .observe(on_control_press::<AlphaSlider>)
                .observe(on_control_release::<AlphaSlider>)
                .observe(on_control_drag_start::<AlphaSlider>)
                .observe(on_control_drag::<AlphaSlider>)
                .observe(on_control_drag_end::<AlphaSlider>);

            parent.spawn((
                ColorInputField {
                    picker: picker_entity,
                    kind: InputFieldKind::Hex,
                },
                combobox_with_selected(vec!["Hex", "RGB", "HSB"], state.input_mode.index()),
            ));

            parent
                .spawn((
                    ColorInputRow(picker_entity),
                    Node {
                        width: percent(100),
                        column_gap: px(12.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|row| {
                    spawn_input_fields(row, picker_entity, state.input_mode, state);
                });
        });
    }
}

struct InputFieldConfig {
    kind: InputFieldKind,
    label: &'static str,
    min: f64,
    max: f64,
    suffix: Option<&'static str>,
}

const INPUT_FIELD_PREFIX_SIZE: f32 = 12.0;

fn spawn_input_fields(
    parent: &mut ChildSpawnerCommands,
    picker_entity: Entity,
    mode: ColorInputMode,
    state: &ColorPickerState,
) {
    let fields: &[InputFieldConfig] = match mode {
        ColorInputMode::Hex => &[InputFieldConfig {
            kind: InputFieldKind::Hex,
            label: "#",
            min: 0.0,
            max: 0.0,
            suffix: None,
        }],
        ColorInputMode::Rgb => &[
            InputFieldConfig {
                kind: InputFieldKind::Red,
                label: "R",
                min: 0.0,
                max: 255.0,
                suffix: None,
            },
            InputFieldConfig {
                kind: InputFieldKind::Green,
                label: "G",
                min: 0.0,
                max: 255.0,
                suffix: None,
            },
            InputFieldConfig {
                kind: InputFieldKind::Blue,
                label: "B",
                min: 0.0,
                max: 255.0,
                suffix: None,
            },
        ],
        ColorInputMode::Hsb => &[
            InputFieldConfig {
                kind: InputFieldKind::Hue,
                label: "H",
                min: 0.0,
                max: 360.0,
                suffix: None,
            },
            InputFieldConfig {
                kind: InputFieldKind::Saturation,
                label: "S",
                min: 0.0,
                max: 100.0,
                suffix: None,
            },
            InputFieldConfig {
                kind: InputFieldKind::Brightness,
                label: "B",
                min: 0.0,
                max: 100.0,
                suffix: None,
            },
        ],
    };

    for config in fields {
        spawn_single_input_field(parent, picker_entity, config, state);
    }

    spawn_single_input_field(
        parent,
        picker_entity,
        &InputFieldConfig {
            kind: InputFieldKind::Alpha,
            label: "A",
            min: 0.0,
            max: 100.0,
            suffix: Some("%"),
        },
        state,
    );
}

fn spawn_single_input_field(
    parent: &mut ChildSpawnerCommands,
    picker_entity: Entity,
    config: &InputFieldConfig,
    state: &ColorPickerState,
) {
    let value = config.kind.format_value(state);
    let is_hex = matches!(config.kind, InputFieldKind::Hex);

    let mut props = TextEditProps::default()
        .with_prefix(TextEditPrefix::Label {
            label: config.label.to_string(),
            size: INPUT_FIELD_PREFIX_SIZE,
        })
        .with_default_value(value);

    if !is_hex {
        props = props
            .numeric_i32()
            .with_min(config.min)
            .with_max(config.max);
    }

    if let Some(suffix) = config.suffix {
        props = props.with_suffix(suffix);
    }

    parent.spawn((
        ColorInputField {
            picker: picker_entity,
            kind: config.kind,
        },
        text_edit(props),
    ));
}

fn handle_trigger_click(
    trigger: On<ButtonClickEvent>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    triggers: Query<&ColorPickerTrigger>,
    mut pickers: Query<&mut ColorPickerState>,
    existing_popovers: Query<(Entity, &ColorPickerPopover)>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
) {
    let Ok(picker_trigger) = triggers.get(trigger.entity) else {
        return;
    };

    let picker_entity = picker_trigger.0;
    let Ok(mut state) = pickers.get_mut(picker_entity) else {
        return;
    };

    for (popover_entity, popover_ref) in &existing_popovers {
        if popover_ref.0 == picker_entity {
            commands.entity(popover_entity).try_despawn();
            state.popover = None;
            if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(trigger.entity) {
                *variant = ButtonVariant::Default;
                set_button_variant(ButtonVariant::Default, &mut bg, &mut border);
            }
            return;
        }
    }

    if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(trigger.entity) {
        *variant = ButtonVariant::ActiveAlt;
        set_button_variant(ButtonVariant::ActiveAlt, &mut bg, &mut border);
    }

    let popover_entity = commands
        .spawn((
            ColorPickerPopover(picker_entity),
            popover(
                PopoverProps::new(trigger.entity)
                    .with_placement(PopoverPlacement::LeftStart)
                    .with_padding(0.0)
                    .with_z_index(150),
            ),
        ))
        .id();

    state.popover = Some(popover_entity);

    commands.entity(popover_entity).with_children(|parent| {
        parent.spawn(popover_header(
            PopoverHeaderProps::new("Color", popover_entity),
            &asset_server,
        ));

        parent.spawn((
            ColorPickerContent(picker_entity),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(CONTENT_PADDING),
                padding: UiRect::all(px(CONTENT_PADDING)),
                width: px(CONTENT_WIDTH + 2.0 * CONTENT_PADDING),
                ..default()
            },
        ));
    });
}

fn handle_popover_closed(
    mut pickers: Query<(Entity, &mut ColorPickerState), With<EditorColorPicker>>,
    triggers: Query<(Entity, &ColorPickerTrigger)>,
    popovers: Query<Entity, With<EditorPopover>>,
    mut button_styles: Query<(&mut BackgroundColor, &mut BorderColor, &mut ButtonVariant)>,
) {
    for (picker_entity, mut state) in &mut pickers {
        let Some(popover_entity) = state.popover else {
            continue;
        };

        if popovers.get(popover_entity).is_ok() {
            continue;
        }

        state.popover = None;

        for (trigger_entity, trigger) in &triggers {
            if trigger.0 != picker_entity {
                continue;
            }
            if let Ok((mut bg, mut border, mut variant)) = button_styles.get_mut(trigger_entity) {
                *variant = ButtonVariant::Default;
                set_button_variant(ButtonVariant::Default, &mut bg, &mut border);
            }
        }
    }
}

fn update_color_picker_visuals(
    pickers: Query<&ColorPickerState, Changed<ColorPickerState>>,
    mut hsv_rect_handles: Query<
        (&HsvRectHandle, &mut Node, &mut BackgroundColor),
        (Without<HueHandle>, Without<AlphaHandle>),
    >,
    mut hue_handles: Query<
        (&HueHandle, &mut Node, &mut BackgroundColor),
        (Without<HsvRectHandle>, Without<AlphaHandle>),
    >,
    mut alpha_handles: Query<
        (&AlphaHandle, &mut Node),
        (Without<HsvRectHandle>, Without<HueHandle>),
    >,
    alpha_handle_material_nodes: Query<(&AlphaHandleMaterial, &MaterialNode<CheckerboardMaterial>)>,
    hsv_rect_material_nodes: Query<(&HsvRectMaterialNode, &MaterialNode<HsvRectMaterial>)>,
    alpha_material_nodes: Query<(&AlphaMaterialNode, &MaterialNode<AlphaSliderMaterial>)>,
    mut hsv_rect_materials: ResMut<Assets<HsvRectMaterial>>,
    mut alpha_materials: ResMut<Assets<AlphaSliderMaterial>>,
    mut checkerboard_materials: ResMut<Assets<CheckerboardMaterial>>,
) {
    for state in &pickers {
        let picker_entity = match hsv_rect_handles
            .iter()
            .find(|(h, _, _)| pickers.get(h.0).is_ok())
        {
            Some((h, _, _)) => h.0,
            None => continue,
        };

        if pickers.get(picker_entity).is_err() {
            continue;
        }

        let current_color = state.to_srgba();

        for (hsv_rect_handle, mut node, mut bg) in &mut hsv_rect_handles {
            if hsv_rect_handle.0 != picker_entity {
                continue;
            }
            node.left = px(state.saturation * CONTENT_WIDTH - HANDLE_SIZE / 2.0);
            node.top = px((1.0 - state.brightness) * CONTENT_WIDTH - HANDLE_SIZE / 2.0);
            bg.0 = current_color.with_alpha(1.0).into();
        }

        for (hue_handle, mut node, mut bg) in &mut hue_handles {
            if hue_handle.0 != picker_entity {
                continue;
            }
            node.left = px((state.hue / 360.0) * CONTENT_WIDTH - HANDLE_SIZE / 2.0);
            let hue_color = hsv_to_rgb(state.hue, 1.0, 1.0);
            bg.0 = Srgba::new(hue_color.0, hue_color.1, hue_color.2, 1.0).into();
        }

        for (alpha_handle, mut node) in &mut alpha_handles {
            if alpha_handle.0 != picker_entity {
                continue;
            }
            node.left = px(state.alpha * CONTENT_WIDTH - HANDLE_SIZE / 2.0);
        }

        for (alpha_handle_mat, material_node) in &alpha_handle_material_nodes {
            if alpha_handle_mat.0 != picker_entity {
                continue;
            }
            if let Some(material) = checkerboard_materials.get_mut(&material_node.0) {
                material.color = Vec4::new(
                    current_color.red,
                    current_color.green,
                    current_color.blue,
                    current_color.alpha,
                );
            }
        }

        for (hsv_rect_mat_node, material_node) in &hsv_rect_material_nodes {
            if hsv_rect_mat_node.0 != picker_entity {
                continue;
            }
            if let Some(material) = hsv_rect_materials.get_mut(&material_node.0) {
                material.hue = state.hue;
            }
        }

        for (alpha_mat_node, material_node) in &alpha_material_nodes {
            if alpha_mat_node.0 != picker_entity {
                continue;
            }
            if let Some(material) = alpha_materials.get_mut(&material_node.0) {
                let (r, g, b) = hsv_to_rgb(state.hue, state.saturation, state.brightness);
                material.color = Vec4::new(r, g, b, 1.0);
            }
        }
    }
}

fn handle_input_field_blur(
    input_focus: Res<InputFocus>,
    mut last_focus: Local<Option<Entity>>,
    mut commands: Commands,
    mut pickers: Query<&mut ColorPickerState>,
    input_fields: Query<&ColorInputField>,
    text_inputs: Query<
        (&bevy_ui_text_input::TextInputBuffer, &ChildOf),
        With<crate::ui::widgets::text_edit::EditorTextEdit>,
    >,
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

    let Ok((buffer, child_of)) = text_inputs.get(blurred_entity) else {
        return;
    };

    let mut current = child_of.parent();
    let mut field_opt = None;

    for _ in 0..10 {
        if let Ok(field) = input_fields.get(current) {
            field_opt = Some(field);
            break;
        }
        if let Ok(parent) = parents.get(current) {
            current = parent.parent();
        } else {
            break;
        }
    }

    let Some(field) = field_opt else {
        return;
    };

    let Ok(mut state) = pickers.get_mut(field.picker) else {
        return;
    };

    let text = buffer.get_text();
    if text.is_empty() {
        return;
    }

    if field.kind.parse_and_apply(&text, &mut state) {
        commands.trigger(ColorPickerChangeEvent {
            entity: field.picker,
            color: state.to_rgba(),
        });
        commands.trigger(ColorPickerCommitEvent {
            entity: field.picker,
            color: state.to_rgba(),
        });
    }
}

fn sync_text_inputs_to_state(
    input_focus: Res<InputFocus>,
    pickers: Query<(Entity, &ColorPickerState), Changed<ColorPickerState>>,
    input_fields: Query<(Entity, &ColorInputField)>,
    mut text_inputs: Query<(Entity, &mut TextInputQueue, &ChildOf), With<EditorTextEdit>>,
    parents: Query<&ChildOf>,
) {
    for (picker_entity, state) in &pickers {
        for (field_entity, field) in &input_fields {
            if field.picker != picker_entity {
                continue;
            }

            let text = field.kind.format_value(state);

            for (text_input_entity, mut queue, child_of) in &mut text_inputs {
                if input_focus.0 == Some(text_input_entity) {
                    continue;
                }

                let mut current = child_of.parent();
                let mut is_descendant = false;

                for _ in 0..10 {
                    if current == field_entity {
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

fn handle_input_mode_change(
    trigger: On<ComboBoxChangeEvent>,
    mut commands: Commands,
    input_fields: Query<&ColorInputField>,
    mut pickers: Query<&mut ColorPickerState>,
    input_rows: Query<(Entity, &ColorInputRow, &Children)>,
    parents: Query<&ChildOf>,
) {
    let mut current = trigger.entity;
    let mut field_opt = None;

    for _ in 0..5 {
        if let Ok(field) = input_fields.get(current) {
            field_opt = Some(field);
            break;
        }
        if let Ok(parent) = parents.get(current) {
            current = parent.parent();
        } else {
            break;
        }
    }

    let Some(field) = field_opt else {
        return;
    };

    if !matches!(field.kind, InputFieldKind::Hex) {
        return;
    }

    let new_mode = ColorInputMode::from_index(trigger.selected);
    let picker_entity = field.picker;

    let Ok(mut state) = pickers.get_mut(picker_entity) else {
        return;
    };

    if state.input_mode == new_mode {
        return;
    }

    state.input_mode = new_mode;

    for (row_entity, row, children) in &input_rows {
        if row.0 != picker_entity {
            continue;
        }

        for child in children.iter() {
            commands.entity(child).try_despawn();
        }

        commands.entity(row_entity).with_children(|parent| {
            spawn_input_fields(parent, picker_entity, new_mode, &state);
        });

        break;
    }
}

fn update_trigger_display(
    pickers: Query<(Entity, &ColorPickerState), Changed<ColorPickerState>>,
    trigger_swatch_materials: Query<(&TriggerSwatchMaterial, &MaterialNode<CheckerboardMaterial>)>,
    mut trigger_labels: Query<(&TriggerLabel, &mut Text)>,
    mut checkerboard_materials: ResMut<Assets<CheckerboardMaterial>>,
) {
    for (picker_entity, state) in &pickers {
        let srgba = state.to_srgba();
        let hex = state.to_hex();

        for (swatch_mat, material_node) in &trigger_swatch_materials {
            if swatch_mat.0 != picker_entity {
                continue;
            }
            if let Some(material) = checkerboard_materials.get_mut(&material_node.0) {
                material.color = Vec4::new(srgba.red, srgba.green, srgba.blue, srgba.alpha);
            }
        }

        for (label, mut text) in &mut trigger_labels {
            if label.0 != picker_entity {
                continue;
            }
            **text = hex.clone();
        }
    }
}

fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * (((b - r) / delta) + 2.0)
    } else {
        60.0 * (((r - g) / delta) + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };

    let s = if max == 0.0 { 0.0 } else { delta / max };
    let v = max;

    (h, s, v)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r + m, g + m, b + m)
}

fn parse_hex(hex: &str) -> Option<[f32; 4]> {
    let hex = hex.trim_start_matches('#');

    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0])
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some([
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                a as f32 / 255.0,
            ])
        }
        _ => None,
    }
}
