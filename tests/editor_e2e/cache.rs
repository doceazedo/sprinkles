use sprinkles_editor::io::EditorCache;

#[test]
fn editor_cache_add_recent_project() {
    let mut cache = EditorCache::default();
    cache.add_recent_project("project_a.ron".to_string());

    assert_eq!(cache.recent_projects.len(), 1);
    assert_eq!(cache.recent_projects[0], "project_a.ron");
    assert_eq!(
        cache.last_opened_project,
        Some("project_a.ron".to_string())
    );
}

#[test]
fn editor_cache_deduplicates_recent_projects() {
    let mut cache = EditorCache::default();
    cache.add_recent_project("project_a.ron".to_string());
    cache.add_recent_project("project_b.ron".to_string());
    cache.add_recent_project("project_a.ron".to_string());

    assert_eq!(cache.recent_projects.len(), 2);
    assert_eq!(
        cache.recent_projects[0], "project_a.ron",
        "most recent should be first"
    );
    assert_eq!(cache.recent_projects[1], "project_b.ron");
}

#[test]
fn editor_cache_limits_recent_projects() {
    let mut cache = EditorCache::default();
    for i in 0..15 {
        cache.add_recent_project(format!("project_{i}.ron"));
    }
    assert_eq!(cache.recent_projects.len(), 10, "should cap at 10");
    assert_eq!(
        cache.recent_projects[0], "project_14.ron",
        "most recent should be first"
    );
}

#[test]
fn editor_cache_remove_recent_project() {
    let mut cache = EditorCache::default();
    cache.add_recent_project("project_a.ron".to_string());
    cache.add_recent_project("project_b.ron".to_string());
    cache.remove_recent_project("project_a.ron");

    assert_eq!(cache.recent_projects.len(), 1);
    assert_eq!(cache.recent_projects[0], "project_b.ron");
}
