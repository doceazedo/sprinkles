use bevy::prelude::*;

pub fn is_descendant_of(entity: Entity, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    let mut current = entity;
    for _ in 0..50 {
        if current == ancestor {
            return true;
        }
        if let Ok(child_of) = parents.get(current) {
            current = child_of.parent();
        } else {
            return false;
        }
    }
    false
}
