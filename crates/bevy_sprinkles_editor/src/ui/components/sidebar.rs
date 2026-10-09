use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::FontSourceTemplate;
use bevy::ui_widgets::{Activate, ActivateOnPress, Button};

use crate::state::{ActiveSidebarTab, SidebarTab};
use crate::ui::tokens::{
    BACKGROUND_COLOR, BORDER_COLOR, CORNER_RADIUS_LG, FONT_PATH, PRIMARY_COLOR, TEXT_BODY_COLOR,
    TEXT_SIZE_SM,
};
use crate::ui::widgets::separator::EditorSeparator;

use super::data_panel::EditorDataPanel;

#[derive(Component, Default, Clone)]
pub struct EditorSidebar;

#[derive(Component, Default, Clone, Copy)]
struct SidebarButton(SidebarTab);

#[derive(Component, Default, Clone)]
struct SidebarButtonIcon;

#[derive(Component, Default, Clone)]
struct SidebarButtonImage;

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (update_sidebar_buttons, toggle_data_panel))
        .add_observer(handle_sidebar_click);
}

pub fn sidebar() -> impl Scene {
    bsn! {
        EditorSidebar
        Node {
            width: px(72),
            flex_direction: { FlexDirection::Column },
            padding: { UiRect::all(px(12)) },
            row_gap: px(12),
            border: { UiRect::right(px(1)) },
        }
        BackgroundColor(BACKGROUND_COLOR)
        BorderColor::all(BORDER_COLOR)
        Children [
            @sidebar_button(SidebarTab::Project)
            --
            @sidebar_button(SidebarTab::Outliner)
            --
            @{EditorSeparator::horizontal()}
            --
            @sidebar_button(SidebarTab::Settings)
        ]
    }
}

fn sidebar_button(tab: SidebarTab) -> impl Scene {
    bsn! {
        SidebarButton(tab)
        Button
        ActivateOnPress
        Hovered
        Node {
            width: percent(100),
            flex_direction: { FlexDirection::Column },
            align_items: { AlignItems::Center },
            row_gap: px(2),
        }
        Children [
            SidebarButton(tab)
            SidebarButtonIcon
            Node {
                width: px(28),
                height: px(28),
                justify_content: { JustifyContent::Center },
                align_items: { AlignItems::Center },
                border_radius: { BorderRadius::all(CORNER_RADIUS_LG) },
            }
            BackgroundColor({ Color::NONE })
            Children [
                SidebarButton(tab)
                SidebarButtonImage
                ImageNode {
                    image: { tab.icon() },
                    color: { Color::Srgba(TEXT_BODY_COLOR) },
                }
                Node {
                    width: px(16),
                    height: px(16),
                }
            ]
            --
            Text({ tab.label() })
            TextFont {
                font: { FontSourceTemplate::Handle(FONT_PATH.into()) },
                font_size: TEXT_SIZE_SM,
            }
            TextColor(TEXT_BODY_COLOR)
        ]
    }
}

fn handle_sidebar_click(
    event: On<Activate>,
    buttons: Query<&SidebarButton, With<Button>>,
    mut active_tab: ResMut<ActiveSidebarTab>,
) {
    if let Ok(sidebar_btn) = buttons.get(event.entity) {
        active_tab.0 = sidebar_btn.0;
    }
}

fn update_sidebar_buttons(
    active_tab: Res<ActiveSidebarTab>,
    buttons: Query<(&SidebarButton, &Hovered), (With<Button>, Without<SidebarButtonIcon>)>,
    changed_hover: Query<(), (Changed<Hovered>, With<SidebarButton>)>,
    mut icon_containers: Query<
        (&SidebarButton, &mut BackgroundColor),
        (With<SidebarButtonIcon>, Without<Button>),
    >,
    mut images: Query<(&SidebarButton, &mut ImageNode), With<SidebarButtonImage>>,
) {
    if !active_tab.is_changed() && changed_hover.is_empty() {
        return;
    }

    for (sidebar_btn, hovered) in &buttons {
        let is_active = active_tab.0 == sidebar_btn.0;
        let is_hovered = hovered.get();

        let (bg_base, bg_alpha) = match (is_active, is_hovered) {
            (false, false) => (TEXT_BODY_COLOR, 0.0),
            (false, true) => (TEXT_BODY_COLOR, 0.05),
            (true, false) => (PRIMARY_COLOR, 0.1),
            (true, true) => (PRIMARY_COLOR, 0.15),
        };

        let icon_color = if is_active {
            PRIMARY_COLOR.lighter(0.05)
        } else {
            TEXT_BODY_COLOR
        };

        for (icon_btn, mut bg) in &mut icon_containers {
            if icon_btn.0 == sidebar_btn.0 {
                bg.0 = bg_base.with_alpha(bg_alpha).into();
            }
        }

        for (img_btn, mut image) in &mut images {
            if img_btn.0 == sidebar_btn.0 {
                image.color = Color::Srgba(icon_color);
            }
        }
    }
}

fn toggle_data_panel(
    active_tab: Res<ActiveSidebarTab>,
    mut data_panels: Query<&mut Node, With<EditorDataPanel>>,
) {
    if !active_tab.is_changed() {
        return;
    }

    let display = if active_tab.0 == SidebarTab::Outliner {
        Display::Flex
    } else {
        Display::None
    };

    for mut node in &mut data_panels {
        node.display = display;
    }
}
