use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::state::{ActiveSidebarTab, SidebarTab};
use crate::ui::tokens::{
    BACKGROUND_COLOR, BORDER_COLOR, CORNER_RADIUS, FONT_PATH, PRIMARY_COLOR, TEXT_BODY_COLOR,
    TEXT_SIZE_SM,
};
use crate::ui::widgets::separator::EditorSeparator;

use super::data_panel::EditorDataPanel;

#[derive(Component)]
pub struct EditorSidebar;

#[derive(Component, Clone, Copy)]
struct SidebarButton(SidebarTab);

#[derive(Component)]
struct SidebarButtonIcon;

#[derive(Component)]
struct SidebarButtonImage;

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            setup_sidebar,
            handle_sidebar_click,
            update_sidebar_buttons,
            toggle_data_panel,
        ),
    );
}

pub fn sidebar() -> impl Bundle {
    (
        EditorSidebar,
        Node {
            width: px(72),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(px(12)),
            row_gap: px(12),
            border: UiRect::right(px(1)),
            ..default()
        },
        BackgroundColor(BACKGROUND_COLOR.into()),
        BorderColor::all(BORDER_COLOR),
    )
}

fn sidebar_button(parent: &mut ChildSpawnerCommands, tab: SidebarTab, asset_server: &AssetServer) {
    let font: Handle<Font> = asset_server.load(FONT_PATH);

    parent
        .spawn((
            SidebarButton(tab),
            Button,
            Hovered::default(),
            Node {
                width: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(2),
                ..default()
            },
        ))
        .with_children(|btn| {
            btn.spawn((
                SidebarButton(tab),
                SidebarButtonIcon,
                Node {
                    width: px(28),
                    height: px(28),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(CORNER_RADIUS),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_child((
                SidebarButton(tab),
                SidebarButtonImage,
                ImageNode::new(asset_server.load(tab.icon()))
                    .with_color(Color::Srgba(TEXT_BODY_COLOR)),
                Node {
                    width: px(16),
                    height: px(16),
                    ..default()
                },
            ));

            btn.spawn((
                Text::new(tab.label()),
                TextFont {
                    font,
                    font_size: TEXT_SIZE_SM,
                    ..default()
                },
                TextColor(TEXT_BODY_COLOR.into()),
            ));
        });
}

fn setup_sidebar(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    sidebars: Query<Entity, Added<EditorSidebar>>,
) {
    for entity in &sidebars {
        commands.entity(entity).with_children(|parent| {
            sidebar_button(parent, SidebarTab::Project, &asset_server);
            sidebar_button(parent, SidebarTab::Outliner, &asset_server);
            parent.spawn(EditorSeparator::horizontal());
            sidebar_button(parent, SidebarTab::Settings, &asset_server);
        });
    }
}

fn handle_sidebar_click(
    interactions: Query<(&Interaction, &SidebarButton), Changed<Interaction>>,
    mut active_tab: ResMut<ActiveSidebarTab>,
) {
    for (interaction, sidebar_btn) in &interactions {
        if *interaction == Interaction::Pressed {
            active_tab.0 = sidebar_btn.0;
        }
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
