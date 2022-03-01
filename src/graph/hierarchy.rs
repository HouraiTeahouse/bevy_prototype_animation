use crate::{
    graph::{application::BoneBinding, AnimationGraph},
    path::EntityPath,
};
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_transform::prelude::{Children, Parent};
use bevy_utils::HashSet;

pub(crate) fn dirty_hierarchy_system(
    mut graphs: Query<&mut AnimationGraph>,
    mut changed: Local<HashSet<Entity>>,
    updated_entities: Query<Entity, Or<(Changed<Parent>, Changed<Name>)>>,
) {
    if updated_entities.is_empty() {
        return;
    }
    changed.extend(updated_entities.iter());
    for mut graph in graphs.iter_mut() {
        if !graph.clips.is_dirty() && graph.clips.is_affected_by(&changed) {
            graph.clips.set_dirty(true);
        }
    }
    changed.clear();
}

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
        let mut hierarchy = HashSet::new();
        for bone in graph.clips.bones_mut() {
            if let Some((entity, seen)) = find_bone(root, &bone.path, &children, &names) {
                commands.entity(entity).insert(BoneBinding {
                    graph: root,
                    bone_id: bone.id,
                });
                bone.set_entity(Some(entity));
                hierarchy.extend(seen);
            } else {
                bone.set_entity(None);
            }
        }
        graph.clips.update_hierarchy(hierarchy);
        graph.clips.set_dirty(false);
    }
}

fn find_bone<'a>(
    root: Entity,
    path: &EntityPath,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Option<(Entity, Vec<Entity>)> {
    let mut seen = Vec::with_capacity(path.len());
    let mut current = root;
    seen.push(root);
    for fragment in path.iter().map(AsRef::as_ref) {
        let mut found = false;
        for child in children.get(current).ok()?.iter() {
            if let Ok(name) = names.get(*child) {
                if name.as_ref() == fragment {
                    found = true;
                    current = *child;
                    seen.push(*child);
                    break;
                }
            }
        }
        if !found {
            return None;
        }
    }
    Some((current, seen))
}
