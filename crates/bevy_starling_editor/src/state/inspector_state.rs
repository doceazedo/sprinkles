use bevy::prelude::*;
use std::collections::HashSet;

#[derive(Resource)]
pub struct InspectorState {
    pub editing_emitter_name: Option<usize>,
    pub collapsed_emitters: HashSet<usize>,
}

impl Default for InspectorState {
    fn default() -> Self {
        Self {
            editing_emitter_name: None,
            collapsed_emitters: HashSet::new(),
        }
    }
}

impl InspectorState {
    pub fn is_emitter_expanded(&self, index: usize) -> bool {
        !self.collapsed_emitters.contains(&index)
    }

    pub fn toggle_emitter(&mut self, index: usize) {
        if self.collapsed_emitters.contains(&index) {
            self.collapsed_emitters.remove(&index);
        } else {
            self.collapsed_emitters.insert(index);
        }
    }
}
