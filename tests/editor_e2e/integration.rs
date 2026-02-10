use super::helpers::*;

use bevy::asset::Assets;
use bevy::prelude::*;
use sprinkles::asset::ParticleSystemAsset;
use sprinkles_editor::state::{DirtyState, EditorState, Inspectable, Inspecting};

#[test]
fn editor_state_tracks_loaded_project() {
    let mut app = create_minimal_app();

    app.init_resource::<EditorState>();
    app.init_resource::<DirtyState>();

    let handle = load_fixture(&mut app, "minimal_particle_system.ron");
    assert!(
        run_until_loaded(&mut app, &handle, 100),
        "fixture should load"
    );

    let mut editor_state = app.world_mut().resource_mut::<EditorState>();
    editor_state.current_project = Some(handle.clone());
    editor_state.inspecting = Some(Inspecting {
        kind: Inspectable::Emitter,
        index: 0,
    });

    let state = app.world().resource::<EditorState>();
    assert!(state.current_project.is_some());
    assert!(state.inspecting.is_some());
    assert_eq!(state.inspecting.unwrap().kind, Inspectable::Emitter);
    assert_eq!(state.inspecting.unwrap().index, 0);
}

#[test]
fn editor_state_switch_to_collider_inspection() {
    let mut app = create_minimal_app();
    app.init_resource::<EditorState>();

    let handle = load_fixture(&mut app, "collision_test.ron");
    assert!(
        run_until_loaded(&mut app, &handle, 100),
        "fixture should load"
    );

    let mut state = app.world_mut().resource_mut::<EditorState>();
    state.current_project = Some(handle.clone());
    state.inspecting = Some(Inspecting {
        kind: Inspectable::Emitter,
        index: 0,
    });

    // switch to inspecting collider 1
    state.inspecting = Some(Inspecting {
        kind: Inspectable::Collider,
        index: 1,
    });

    let state = app.world().resource::<EditorState>();
    let inspecting = state.inspecting.unwrap();
    assert_eq!(inspecting.kind, Inspectable::Collider);
    assert_eq!(inspecting.index, 1);
}

#[test]
fn dirty_state_tracks_unsaved_changes() {
    let mut app = create_minimal_app();
    app.init_resource::<DirtyState>();

    {
        let dirty = app.world().resource::<DirtyState>();
        assert!(!dirty.has_unsaved_changes);
    }

    app.world_mut().resource_mut::<DirtyState>().has_unsaved_changes = true;

    let dirty = app.world().resource::<DirtyState>();
    assert!(dirty.has_unsaved_changes);
}

#[test]
fn editor_state_inspecting_emitter_index_matches_asset() {
    let mut app = create_minimal_app();
    app.init_resource::<EditorState>();

    let handle = load_fixture(&mut app, "two_emitters.ron");
    assert!(run_until_loaded(&mut app, &handle, 100));

    let assets = app.world().resource::<Assets<ParticleSystemAsset>>();
    let asset = assets.get(&handle).expect("asset should be loaded");
    let emitter_count = asset.emitters.len();
    assert_eq!(emitter_count, 2);

    let mut state = app.world_mut().resource_mut::<EditorState>();
    state.current_project = Some(handle.clone());

    // inspect second emitter (index 1)
    state.inspecting = Some(Inspecting {
        kind: Inspectable::Emitter,
        index: 1,
    });

    let state = app.world().resource::<EditorState>();
    let assets = app.world().resource::<Assets<ParticleSystemAsset>>();
    let asset = assets.get(state.current_project.as_ref().unwrap()).unwrap();
    let idx = state.inspecting.unwrap().index as usize;
    assert!(idx < asset.emitters.len());
    assert_eq!(asset.emitters[idx].name, "Emitter B");
}
