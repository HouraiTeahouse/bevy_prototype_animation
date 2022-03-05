use bevy_core::Name;
use bevy_reflect::TypeRegistry;
use std::any::TypeId;
use std::cmp::Ordering;
use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

mod field;
pub use field::{FieldPath, ReflectPathError};

/// A named path through a hierarchy of entities.
///
/// This represents a String-like path taking the form of "root/a/b/c/d". When parsing,
/// this type will skip any preceding backslashes, so `////root//hips` is the same as
/// `root//hips`.
///
/// This type comes pre-split into individual levels, unlike a normal string.
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct EntityPath {
    parts: Box<[Name]>,
}

impl EntityPath {
    const SEPERATOR: &'static str = "/";

    pub fn from_parts(parts: Vec<Name>) -> Self {
        Self {
            parts: parts.into_boxed_slice(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Name> {
        self.parts.iter()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.parts.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

impl FromStr for EntityPath {
    type Err = Infallible;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_parts(
            src.split(Self::SEPERATOR)
                .into_iter()
                .skip_while(|part| part.is_empty())
                .map(|part| Name::new(part.to_string()))
                .collect(),
        ))
    }
}

impl fmt::Display for EntityPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, part) in self.parts.iter().enumerate() {
            if idx > 0 {
                f.write_str(Self::SEPERATOR)?;
            }
            f.write_str(part.as_ref())?;
        }
        Ok(())
    }
}

/// A named field path through a component type.
///
/// This represents a String-like path taking the form of "root.a.b.c.d".
///
/// This type comes pre-split into individual levels, unlike a normal string.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct AccessPath {
    component_type_id: TypeId,
    component_name: String,
    field_path: FieldPath,
}

impl AccessPath {
    const SEPERATOR: &'static str = ".";

    pub fn parse<'a>(
        registry: &'a TypeRegistry,
        path: &'a str,
    ) -> Result<Self, ParsePathError<'a>> {
        let (component, field) = path
            .split_once(Self::SEPERATOR)
            .ok_or(ParsePathError::NoComponentName)?;
        let registration = registry
            .get_with_name(component)
            .ok_or(ParsePathError::InvalidComponentType)?;
        Ok(Self {
            component_type_id: registration.type_id(),
            component_name: component.to_string(),
            field_path: FieldPath::parse(field)?,
        })
    }

    pub fn component_type_id(&self) -> TypeId {
        self.component_type_id
    }

    pub fn component_name(&self) -> &str {
        self.component_name.as_ref()
    }

    pub fn field_path(&self) -> &FieldPath {
        &self.field_path
    }
}

impl fmt::Display for AccessPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.component_name.as_ref())?;
        f.write_str(Self::SEPERATOR)?;
        self.field_path.fmt(f)
    }
}

impl PartialOrd for AccessPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // As the component name is going to be the same if the type ID
        // is the same, order on the type ID instead.
        Some(
            self.component_type_id
                .cmp(&other.component_type_id)
                .then(self.field_path.cmp(&other.field_path)),
        )
    }
}

impl Ord for AccessPath {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

/// A full property path selecting a single field within a hierarchy of
/// entities. Comprised of a [`EntityPath`] followed by a [`FieldPath`].
/// Each part of the full path is accessible separately.
///
/// This represents a String-like path taking the form of "root/a/b/c/@droot.a.b.c.d".
/// Each part of the path is delimited by a "@".
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct PropertyPath {
    entity: EntityPath,
    access: AccessPath,
}

impl PropertyPath {
    const SEPERATOR: char = '@';

    pub fn parse<'a>(
        registry: &'a TypeRegistry,
        path: &'a str,
    ) -> Result<Self, ParsePathError<'a>> {
        let (entity, access) = path
            .split_once(Self::SEPERATOR)
            .ok_or(ParsePathError::MissingDelimiter)?;
        Ok(Self::from_parts(
            EntityPath::from_str(entity).unwrap(),
            AccessPath::parse(registry, access)?,
        ))
    }

    /// Constructs a [`PropertyPath`] from it's consistituent parts.
    pub fn from_parts(entity: EntityPath, access: AccessPath) -> Self {
        Self { entity, access }
    }

    /// Splits the property path into it's constituent parts.
    pub fn into_parts(self) -> (EntityPath, AccessPath) {
        (self.entity, self.access)
    }

    /// Gets a immutable reference to the [`AccessPath`] in the full property path.
    pub fn entity(&self) -> &EntityPath {
        &self.entity
    }

    /// Gets immutable reference to the [`AccessPath`] in the full property path.
    pub fn access(&self) -> &AccessPath {
        &self.access
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParsePathError<'a> {
    MissingDelimiter,
    InvalidComponentType,
    NoComponentName,
    InvalidFieldPath(ReflectPathError<'a>),
}

impl<'a> From<ReflectPathError<'a>> for ParsePathError<'a> {
    fn from(value: ReflectPathError<'a>) -> Self {
        Self::InvalidFieldPath(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy_ecs::prelude::*;
    use bevy_reflect::prelude::*;

    #[derive(Component, Reflect)]
    struct Test {
        a: u32,
        b: u32,
        c: u32,
    }

    #[test]
    pub fn test_parse_entity_path() {
        let path_str = "a/b/c/d/e/f//g";
        let path = EntityPath::from_str(path_str).unwrap();
        let vec: Vec<_> = path.iter().map(AsRef::as_ref).collect();
        assert_eq!(vec, vec!["a", "b", "c", "d", "e", "f", "", "g"]);
    }

    #[test]
    pub fn test_parse_entity_path_ignore_leading_backslash() {
        let path_str = "///a/b/c/dead/e/f//g";
        let path = EntityPath::from_str(path_str).unwrap();
        let vec: Vec<_> = path.iter().map(AsRef::as_ref).collect();
        assert_eq!(vec, vec!["a", "b", "c", "dead", "e", "f", "", "g"]);
    }

    #[test]
    pub fn test_parse_access_path() {
        let mut registry = TypeRegistry::default();
        registry.register::<Test>();
        let path_str = "bevy_prototype_animation::path::test::Test.b.c.d.e.f.g";
        let path = AccessPath::parse(&registry, path_str).unwrap();
        let path = path.to_string();
        assert_eq!(
            path.as_str(),
            "bevy_prototype_animation::path::test::Test.b.c.d.e.f.g",
        );
    }

    #[test]
    pub fn test_parse_access_path_fails_on_empty_field() {
        let mut registry = TypeRegistry::default();
        registry.register::<Test>();
        let path_str = "bevy_prototype_animation::path::test::Test.b.c.d.e.f..g";
        let path = AccessPath::parse(&registry, path_str);
        assert_eq!(
            path,
            Err(ParsePathError::InvalidFieldPath(
                ReflectPathError::ExpectedIdent { index: 10 }
            ))
        );
    }

    #[test]
    pub fn test_parse_access_path_invalid_typek() {
        let registry = TypeRegistry::default();
        let path_str = "bevy_prototype_animation::path::test::Test.b.c.d.e.f a.g";
        let path = AccessPath::parse(&registry, path_str);
        assert_eq!(path, Err(ParsePathError::InvalidComponentType));
    }

    #[test]
    pub fn test_parse_property_path() {
        let mut registry = TypeRegistry::default();
        registry.register::<Test>();
        let path_str = "a/b/c/d/e/f//g@bevy_prototype_animation::path::test::Test.b.c.d.e.f.g";
        let path = PropertyPath::parse(&registry, path_str).unwrap();
        let entity_vec: Vec<_> = path.entity().iter().map(AsRef::as_ref).collect();
        let field = path.access().to_string();
        assert_eq!(entity_vec, vec!["a", "b", "c", "d", "e", "f", "", "g"]);
        assert_eq!(
            field.as_str(),
            "bevy_prototype_animation::path::test::Test.b.c.d.e.f.g",
        );
    }

    #[test]
    pub fn test_parse_property_path_works_with_empty_entity() {
        let mut registry = TypeRegistry::default();
        registry.register::<Test>();
        let path_str = "@bevy_prototype_animation::path::test::Test.b.c.d.e.f.g";
        let path = PropertyPath::parse(&registry, path_str).unwrap();
        let entity_vec: Vec<_> = path.entity().iter().collect();
        let field = path.access().to_string();
        assert!(entity_vec.is_empty());
        assert_eq!(
            field.as_str(),
            "bevy_prototype_animation::path::test::Test.b.c.d.e.f.g",
        );
    }

    #[test]
    pub fn test_parse_property_path_fails_on_empty_field() {
        let mut registry = TypeRegistry::default();
        registry.register::<Test>();
        let path_str = "a/b/c/d/e/f//g@bevy_prototype_animation::path::test::Test.b.c.d.e.f..g";
        let path = PropertyPath::parse(&registry, path_str);
        assert_eq!(
            path,
            Err(ParsePathError::InvalidFieldPath(
                ReflectPathError::ExpectedIdent { index: 10 }
            ))
        );
    }
}
