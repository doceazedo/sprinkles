use bevy::prelude::*;
use bevy::text::FontSourceTemplate;

use crate::ui::icons::{ICON_ADD, ICON_ARROW_DOWN};
use crate::ui::tokens::{BORDER_COLOR, FONT_PATH, TEXT_DISPLAY_COLOR, TEXT_SIZE};
use crate::ui::widgets::button::{ButtonClickEvent, ButtonVariant, IconButtonProps, icon_button};

pub fn plugin(app: &mut App) {
    app.add_systems(Update, setup_panel_section_buttons);
}

#[derive(Component, Default, Clone)]
pub struct EditorPanelSection;

#[derive(Component, Default, Clone)]
struct PanelSectionHeader;

#[derive(Component, Default, Clone)]
struct PanelSectionButtonsContainer;

#[derive(Component)]
pub struct PanelSectionAddButton(pub Entity);

#[derive(Component)]
struct PanelSectionCollapseButton(Entity);

#[derive(Component, Default, Clone)]
struct Collapsed(bool);

#[derive(Component, Default, Clone)]
struct PanelSectionState {
    has_add_button: bool,
    collapsible: bool,
}

#[derive(Default, Clone, Copy)]
pub enum PanelSectionSize {
    #[default]
    MD,
    XL,
}

impl PanelSectionSize {
    fn padding(&self) -> UiRect {
        match self {
            Self::MD => UiRect::all(px(12)),
            Self::XL => UiRect::new(px(24), px(24), px(14), px(24)),
        }
    }
}

#[derive(Default)]
pub struct PanelSectionProps {
    pub title: String,
    pub size: PanelSectionSize,
    pub has_add_button: bool,
    pub collapsible: bool,
}

impl PanelSectionProps {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..default()
        }
    }

    pub fn with_size(mut self, size: PanelSectionSize) -> Self {
        self.size = size;
        self
    }

    pub fn with_add_button(mut self) -> Self {
        self.has_add_button = true;
        self
    }

    pub fn collapsible(mut self) -> Self {
        self.collapsible = true;
        self
    }
}

pub fn panel_section(props: PanelSectionProps) -> impl Scene {
    let PanelSectionProps {
        title,
        size,
        has_add_button,
        collapsible,
    } = props;
    let padding = size.padding();

    bsn! {
        EditorPanelSection
        Collapsed
        Node {
            width: percent(100),
            flex_direction: { FlexDirection::Column },
            row_gap: px(12),
            padding: { padding },
            border: { UiRect::bottom(px(1)) },
        }
        template_value(BorderColor::all(BORDER_COLOR))
        template_value(PanelSectionState {
            has_add_button,
            collapsible,
        })
        Children [
            (
                PanelSectionHeader
                Node {
                    width: percent(100),
                    justify_content: { JustifyContent::SpaceBetween },
                    align_items: { AlignItems::Center },
                }
                Children [
                    (
                        Text({ title })
                        TextFont {
                            font: { FontSourceTemplate::Handle(FONT_PATH.into()) },
                            font_size: TEXT_SIZE,
                            weight: { FontWeight::SEMIBOLD },
                        }
                        TextColor({ TEXT_DISPLAY_COLOR })
                    ),
                    (
                        PanelSectionButtonsContainer
                        Node {
                            align_items: { AlignItems::Center },
                        }
                    ),
                ]
            )
        ]
    }
}

fn setup_panel_section_buttons(
    mut commands: Commands,
    _asset_server: Res<AssetServer>,
    new_sections: Query<(Entity, &PanelSectionState, &Children), Added<EditorPanelSection>>,
    headers: Query<&Children, With<PanelSectionHeader>>,
    containers: Query<Entity, With<PanelSectionButtonsContainer>>,
) {
    for (section_entity, state, section_children) in &new_sections {
        let Some(&header_entity) = section_children.first() else {
            continue;
        };
        let Ok(header_children) = headers.get(header_entity) else {
            continue;
        };
        let Some(&container_entity) = header_children.get(1) else {
            continue;
        };
        if containers.get(container_entity).is_err() {
            continue;
        }

        if state.has_add_button {
            let add_entity = commands
                .spawn_scene(icon_button(
                    IconButtonProps::new(ICON_ADD).variant(ButtonVariant::Ghost),
                ))
                .insert(PanelSectionAddButton(section_entity))
                .observe(on_add_click)
                .id();
            commands.entity(container_entity).add_child(add_entity);
        }

        if state.collapsible {
            let collapse_entity = commands
                .spawn_scene(icon_button(
                    IconButtonProps::new(ICON_ARROW_DOWN).variant(ButtonVariant::Ghost),
                ))
                .insert((
                    PanelSectionCollapseButton(section_entity),
                    UiTransform {
                        rotation: Rot2::degrees(180.0),
                        ..default()
                    },
                ))
                .observe(on_collapse_click)
                .id();
            commands.entity(container_entity).add_child(collapse_entity);
        }
    }
}

fn on_add_click(
    event: On<ButtonClickEvent>,
    add_buttons: Query<&PanelSectionAddButton>,
    mut commands: Commands,
) {
    let Ok(add_button) = add_buttons.get(event.entity) else {
        return;
    };
    commands.trigger(ButtonClickEvent {
        entity: add_button.0,
    });
}

fn on_collapse_click(
    event: On<ButtonClickEvent>,
    collapse_buttons: Query<&PanelSectionCollapseButton>,
    mut sections: Query<(&mut Collapsed, &Children), With<EditorPanelSection>>,
    mut nodes: Query<&mut Node, Without<PanelSectionHeader>>,
    headers: Query<Entity, With<PanelSectionHeader>>,
    mut button_transforms: Query<&mut UiTransform>,
) {
    let button_entity = event.entity;
    let Ok(collapse_button) = collapse_buttons.get(button_entity) else {
        return;
    };

    let Ok((mut collapsed, section_children)) = sections.get_mut(collapse_button.0) else {
        return;
    };

    collapsed.0 = !collapsed.0;

    for child in section_children.iter() {
        if headers.get(child).is_ok() {
            continue;
        }
        if let Ok(mut node) = nodes.get_mut(child) {
            node.display = if collapsed.0 {
                Display::None
            } else {
                Display::Flex
            };
        }
    }

    if let Ok(mut transform) = button_transforms.get_mut(button_entity) {
        transform.rotation = if collapsed.0 {
            Rot2::degrees(0.0)
        } else {
            Rot2::degrees(180.0)
        };
    }
}
