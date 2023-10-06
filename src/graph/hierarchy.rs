use crate::{
    graph::{application::BoneBinding, AnimationGraph},
    path::EntityPath,
};
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_hierarchy::Children;

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
    for fragment in path.iter() {
        let mut found = false;
        for child in children.get(current).ok()?.iter() {
            if let Ok(name) = names.get(*child) {
                if name == fragment {
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
