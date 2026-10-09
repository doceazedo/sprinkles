use bevy::prelude::*;

use crate::project::SaveProjectEvent;
use crate::ui::components::playback_controls::playback_controls;
use crate::ui::components::project_selector::project_selector;
use crate::ui::components::seekbar::seekbar;
use crate::ui::tokens::{BACKGROUND_COLOR, BORDER_COLOR};
use crate::ui::widgets::button::{ButtonClickEvent, ButtonProps, ButtonVariant, button};
use crate::ui::widgets::separator::EditorSeparator;

#[derive(Component, Default, Clone)]
pub struct SaveButton;

fn on_save_button_click(_event: On<ButtonClickEvent>, mut commands: Commands) {
    commands.trigger(SaveProjectEvent);
}

#[derive(Component, Default, Clone)]
pub struct EditorTopbar;

pub fn topbar() -> impl Scene {
    bsn! {
        EditorTopbar
        Node {
            width: percent(100),
            height: px(52),
            padding: { UiRect::all(px(12)) },
            border: { UiRect::bottom(px(1)) },
            justify_content: { JustifyContent::SpaceBetween },
            align_items: { AlignItems::Center },
        }
        BackgroundColor(BACKGROUND_COLOR)
        BorderColor::all(BORDER_COLOR)
        Children [
            @project_selector()
            --
            Node {
                column_gap: px(12),
                align_items: { AlignItems::Center },
            }
            Children [
                @seekbar()
                --
                @playback_controls()
                --
                @{EditorSeparator::vertical()}
                --
                SaveButton
                @button(ButtonProps::new("Save").with_variant(ButtonVariant::Primary))
                on(on_save_button_click)
            ]
        ]
    }
}
