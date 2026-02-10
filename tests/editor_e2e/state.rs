use sprinkles_editor::state::{DirtyState, EditorState, Inspectable, Inspecting};

#[test]
fn editor_state_defaults_to_no_project() {
    let state = EditorState::default();
    assert!(state.current_project.is_none());
    assert!(state.current_project_path.is_none());
    assert!(state.inspecting.is_none());
}

#[test]
fn dirty_state_defaults_to_clean() {
    let state = DirtyState::default();
    assert!(!state.has_unsaved_changes);
}

#[test]
fn inspecting_stores_emitter_selection() {
    let inspecting = Inspecting {
        kind: Inspectable::Emitter,
        index: 2,
    };
    assert_eq!(inspecting.kind, Inspectable::Emitter);
    assert_eq!(inspecting.index, 2);
}

#[test]
fn inspecting_stores_collider_selection() {
    let inspecting = Inspecting {
        kind: Inspectable::Collider,
        index: 0,
    };
    assert_eq!(inspecting.kind, Inspectable::Collider);
    assert_eq!(inspecting.index, 0);
}

#[test]
fn inspectable_equality() {
    assert_eq!(Inspectable::Emitter, Inspectable::Emitter);
    assert_eq!(Inspectable::Collider, Inspectable::Collider);
    assert_ne!(Inspectable::Emitter, Inspectable::Collider);
}
