use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::ui::tokens::{BORDER_COLOR, FONT_PATH, TEXT_BODY_COLOR, TEXT_SIZE};

const ICON_CHECK: &str = "icons/ri-check-fill.png";

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (handle_checkbox_hover, handle_checkbox_click));
}

#[derive(Component)]
pub struct EditorCheckbox;

#[derive(Component, Default)]
pub struct CheckboxState {
    pub checked: bool,
}

#[derive(Component)]
struct CheckboxIcon;

#[derive(Component)]
struct CheckboxBox;

#[derive(Default)]
pub struct CheckboxProps {
    pub label: String,
    pub checked: bool,
}

impl CheckboxProps {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ..default()
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
}

pub fn checkbox(props: CheckboxProps, asset_server: &AssetServer) -> impl Bundle {
    let CheckboxProps { label, checked } = props;
    let font: Handle<Font> = asset_server.load(FONT_PATH);
    let icon_display = if checked {
        Display::Flex
    } else {
        Display::None
    };

    (
        EditorCheckbox,
        CheckboxState { checked },
        Button,
        Hovered::default(),
        Node {
            align_items: AlignItems::Center,
            column_gap: px(6),
            ..default()
        },
        children![
            (
                CheckboxBox,
                Node {
                    width: px(16),
                    height: px(16),
                    border: UiRect::all(px(1.0)),
                    border_radius: BorderRadius::all(px(2.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BorderColor::all(BORDER_COLOR),
                children![(
                    CheckboxIcon,
                    ImageNode::new(asset_server.load(ICON_CHECK))
                        .with_color(Color::Srgba(TEXT_BODY_COLOR)),
                    Node {
                        width: px(12),
                        height: px(12),
                        display: icon_display,
                        ..default()
                    },
                )],
            ),
            (
                Text::new(label),
                TextFont {
                    font: font.into(),
                    font_size: TEXT_SIZE,
                    ..default()
                },
                TextColor(TEXT_BODY_COLOR.into()),
            ),
        ],
    )
}

fn handle_checkbox_hover(
    checkboxes: Query<(&Hovered, &Children), (Changed<Hovered>, With<EditorCheckbox>)>,
    mut boxes: Query<&mut BorderColor, With<CheckboxBox>>,
) {
    for (hovered, children) in &checkboxes {
        let Some(&box_entity) = children.first() else {
            continue;
        };

        let Ok(mut border_color) = boxes.get_mut(box_entity) else {
            continue;
        };

        let border = if hovered.get() {
            BORDER_COLOR.lighter(0.05)
        } else {
            BORDER_COLOR
        };
        *border_color = BorderColor::all(border);
    }
}

fn handle_checkbox_click(
    mut checkboxes: Query<
        (&Interaction, &mut CheckboxState, &Children),
        (Changed<Interaction>, With<EditorCheckbox>),
    >,
    boxes: Query<&Children, With<CheckboxBox>>,
    mut icons: Query<&mut Node, With<CheckboxIcon>>,
) {
    for (interaction, mut state, checkbox_children) in &mut checkboxes {
        if *interaction != Interaction::Pressed {
            continue;
        }

        state.checked = !state.checked;

        let Some(&box_entity) = checkbox_children.first() else {
            continue;
        };

        let Ok(box_children) = boxes.get(box_entity) else {
            continue;
        };

        let Some(&icon_entity) = box_children.first() else {
            continue;
        };

        let Ok(mut icon_node) = icons.get_mut(icon_entity) else {
            continue;
        };

        icon_node.display = if state.checked {
            Display::Flex
        } else {
            Display::None
        };
    }
}
