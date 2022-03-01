use crate::{
    graph::{application::BoneBinding, AnimationGraph},
    path::EntityPath,
};
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_transform::prelude::{Children, Parent, PreviousParent};
use bevy_utils::HashSet;

// This system bubbles up changes in transform hierarchies, dirtying all
// affected animation graphs. This does `O(d)` parent lookupps, where `d`
// is the depth of the changed
pub(crate) fn dirty_hierarchy_system(
    mut graphs: Query<&mut AnimationGraph>,
    changed: Query<(Entity, Option<&PreviousParent>), Or<(Changed<Parent>, Changed<Name>)>>,
//     removed: Query<&PreviousParent, Without<Parent>>,
    parents: Query<&Parent, With<Name>>,
    mut open_set: Local<Vec<Entity>>,
    mut visited: Local<HashSet<Entity>>,
) {
    // Check both the current entity's hierarchy and its previous one
    // as it may have moved into or out of a animator hierarchy.
//     open_set.extend(removed.iter().map(|pp| pp.0));
    for (entity, previous) in changed.iter() {
        open_set.push(entity);
        if let Some(prev) = previous {
        //     open_set.push(prev.0);
        }
    }
    // Bubble up change and mark all graphs in the ancestor path as dirty
    while let Some(current) = open_set.pop() {
        // Reduce the traversal by avoiding paths already traversed.
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);
        if let Ok(mut graph) = graphs.get_mut(current) {
            graph.clips.set_dirty(true);
        }
        if let Ok(parent) = parents.get(current) {
            open_set.push(parent.0);
        }
    }
    visited.clear();
}

// This runs a `O(n*b*d)` operation for every animation graph in the World.
// Here, n is the number of bones the graph has, b is the upper bound branching
// factor of the hierarchy, and d is the deepest bone in the hierarchy.
//
// This will run on a given graph any time a descendant entity's Parent or Name
// components are changed/added, despawned, or when new clips added to a graph
// that creates new bones. Ideally graphs should only have this done once during
// initialization.
pub(crate) fn bind_hierarchy_system(
    mut graphs: Query<(Entity, &mut AnimationGraph), Changed<AnimationGraph>>,
    children: Query<&Children>,
    names: Query<&Name>,
    mut commands: Commands,
) {
    for (root, mut graph) in graphs.iter_mut() {
        if !graph.clips.is_dirty() {
            continue;
        }
        for bone in graph.clips.bones_mut() {
            if let Some(entity) = find_bone(root, &bone.path, &children, &names) {
                commands.entity(entity).insert(BoneBinding {
                    graph: root,
                    bone_id: bone.id,
                });
                bone.set_entity(Some(entity));
            } else {
                bone.set_entity(None);
            }
        }
        graph.clips.set_dirty(false);
    }
}

fn find_bone<'a>(
    root: Entity,
    path: &EntityPath,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Option<Entity> {
    let mut current = root;
    for fragment in path.iter().map(AsRef::as_ref) {
	let fragment = Name::new(fragment.to_string());
        let mut found = false;
        for child in children.get(current).ok()?.iter() {
            if let Ok(name) = names.get(*child) {
                if name == &fragment {
                    found = true;
                    current = *child;
                    break;
                }
            }
        }
        if !found {
            return None;
        }
    }
    Some(current)
}
