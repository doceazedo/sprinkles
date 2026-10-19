use std::sync::Mutex;

#[cfg(not(debug_assertions))]
use bevy::ecs::error::{BevyError, ErrorContext, error};
use bevy::prelude::*;

use crate::ui::components::toasts::ToastEvent;

static PENDING_ERRORS: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn plugin(app: &mut App) {
    app.add_systems(Update, show_pending_errors);
}

#[cfg(not(debug_assertions))]
pub fn log_and_toast(err: BevyError, ctx: ErrorContext) {
    if let Ok(mut pending) = PENDING_ERRORS.lock() {
        pending.push(err.to_string());
    }
    error(err, ctx);
}

fn show_pending_errors(mut commands: Commands) {
    let Ok(mut pending) = PENDING_ERRORS.lock() else {
        return;
    };
    for message in pending.drain(..) {
        commands.trigger(ToastEvent::error(format!(
            "Something went wrong: {message}"
        )));
    }
}
