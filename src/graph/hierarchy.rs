use crate::graph::{application::BoneBinding, AnimationGraph};
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_transform::prelude::Children;

fn bind_animation_graph_hierarchy_system(
    mut graphs: Query<(Entity, &mut AnimationGraph), Changed<AnimationGraph>>,
    children: Query<&Children>,
    names: Query<&Name>,
    mut commands: Commands,
) {
    for (root, mut graph) in graphs.iter_mut() {
        // TODO: Add a dirty check here for hierarchy rebuilds.
        for bone in graph.bones_mut() {
            let path = bone.path.iter().map(AsRef::as_ref);
            let entity = find_bone(root, path, &children, &names);
            bone.set_entity(entity);
            if let Some(entity) = entity {
                commands.entity(entity).insert(BoneBinding {
                    graph: root,
                    bone_id: bone.id,
                });
            }
        }
    }
}

fn find_bone<'a>(
    root: Entity,
    mut path: impl Iterator<Item = &'a str>,
    children: &Query<&Children>,
    names: &Query<&Name>,
) -> Option<Entity> {
    let mut current = root;
    for fragment in path {
        let mut found = false;
        for child in children.get(current).ok()?.iter() {
            if let Ok(name) = names.get(*child) {
                if name.as_ref() == fragment {
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
