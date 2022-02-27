use crate::graph::{track::BoneId, AnimationGraph};
use bevy_ecs::prelude::*;
use bevy_log::warn;
use bevy_reflect::{ReflectMut, TypeRegistryArc};
use std::ops::Deref;

#[derive(Component)]
pub(crate) struct BoneBinding {
    graph: Entity,
    bone_id: BoneId,
}

// This MUST be used as an exclusive system for aliasing safety.
fn animate_entities_system(
    world: &mut World,
    entities: Query<(Entity, &BoneBinding)>,
    graphs: Query<&AnimationGraph>,
    type_registry: Res<TypeRegistryArc>,
    mut commands: Commands,
) {
    // TODO: Consider parallelizing this via ReflectComponent::reflect_component_unchecked_mut
    for (entity, binding) in entities.iter() {
        let result = animate_entity(entity, binding, &graphs, &type_registry, world);
        if result.is_none() {
            commands.entity(entity).remove::<BoneBinding>();
        }
    }
}

fn animate_entity(
    entity: Entity,
    binding: &BoneBinding,
    graphs: &Query<&AnimationGraph>,
    type_registry: &TypeRegistryArc,
    world: &mut World,
) -> Option<()> {
    let graph = graphs.get(binding.graph).ok()?;
    let bone = graph.get_bone(binding.bone_id)?;
    if bone.entity() != Some(entity) {
        return None;
    }

    let type_registry = type_registry.read();
    for property in bone.properties() {
        let component_name = property.component_name();
        let mut component = type_registry
            .get_with_name(property.component_name())
            .and_then(|registration| registration.data::<ReflectComponent>())
            .and_then(|reflect| reflect.reflect_component_mut(world, entity));

        if let Some(mut comp) = component {
            match comp.reflect_mut() {
                ReflectMut::Struct(component) => {
                    if let Some(field) = component.field_mut(&property.field_path()) {
                        bone.sample_property(property, &graph.state, field);
                    } else {
                        warn!(
                            "Failed to animate '{}'. Struct '{}' has no field {}.",
                            property.deref(),
                            property.component_name(),
                            property.field_path(),
                        );
                    }
                }
                _ => {
                    warn!(
			"Failed to animate '{}'. Non-struct components currently cannot be animated.",
			property.deref(),
			);
                }
            }
        }
    }
    Some(())
}
