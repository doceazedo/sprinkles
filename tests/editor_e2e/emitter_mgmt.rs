use super::helpers::*;

use sprinkles::asset::EmitterData;
use sprinkles_editor::state::{EditorState, Inspectable, Inspecting};

#[test]
fn test_add_emitter() {
    let mut asset = create_test_asset(&["Emitter 1"]);
    assert_eq!(asset.emitters.len(), 1);

    asset.emitters.push(EmitterData {
        name: "Emitter 2".to_string(),
        ..Default::default()
    });

    assert_eq!(asset.emitters.len(), 2);
    assert_eq!(asset.emitters[1].name, "Emitter 2");
}

#[test]
fn test_add_emitter_unique_name() {
    let asset = create_test_asset(&["Emitter"]);
    let existing: Vec<&str> = asset.emitters.iter().map(|e| e.name.as_str()).collect();

    let name = next_unique_name("Emitter", &existing);
    assert_eq!(name, "Emitter 2");

    let existing_with_2 = vec!["Emitter", "Emitter 2"];
    let name = next_unique_name("Emitter", &existing_with_2);
    assert_eq!(name, "Emitter 3");
}

#[test]
fn test_remove_emitter() {
    let mut asset = create_test_asset(&["Fire", "Smoke", "Sparks"]);
    assert_eq!(asset.emitters.len(), 3);

    asset.emitters.remove(1);

    assert_eq!(asset.emitters.len(), 2);
    assert_eq!(asset.emitters[0].name, "Fire");
    assert_eq!(asset.emitters[1].name, "Sparks");
}

#[test]
fn test_duplicate_emitter() {
    let mut asset = create_test_asset(&["Fire"]);
    asset.emitters[0].time.lifetime = 5.0;
    asset.emitters[0].emission.particles_amount = 100;

    let mut cloned = asset.emitters[0].clone();
    let existing: Vec<&str> = asset.emitters.iter().map(|e| e.name.as_str()).collect();
    cloned.name = next_unique_name("Fire", &existing);
    asset.emitters.insert(1, cloned);

    assert_eq!(asset.emitters.len(), 2);
    assert_eq!(asset.emitters[1].name, "Fire 2");
    assert_eq!(asset.emitters[1].time.lifetime, 5.0);
    assert_eq!(asset.emitters[1].emission.particles_amount, 100);
}

#[test]
fn test_rename_emitter() {
    let mut asset = create_test_asset(&["Emitter 1"]);
    asset.emitters[0].name = "Fire".to_string();
    assert_eq!(asset.emitters[0].name, "Fire");
}

#[test]
fn test_select_emitter_updates_inspecting() {
    let asset = create_test_asset(&["Alpha", "Beta", "Gamma"]);
    let mut state = EditorState::default();

    state.inspecting = Some(Inspecting {
        kind: Inspectable::Emitter,
        index: 0,
    });
    assert_eq!(
        asset.emitters[state.inspecting.unwrap().index as usize].name,
        "Alpha"
    );

    state.inspecting = Some(Inspecting {
        kind: Inspectable::Emitter,
        index: 2,
    });
    assert_eq!(
        asset.emitters[state.inspecting.unwrap().index as usize].name,
        "Gamma"
    );
}

#[test]
fn test_emitter_enabled_toggle() {
    let mut asset = create_test_asset(&["Emitter"]);
    assert!(asset.emitters[0].enabled);

    asset.emitters[0].enabled = false;
    assert!(!asset.emitters[0].enabled);

    asset.emitters[0].enabled = true;
    assert!(asset.emitters[0].enabled);
}
