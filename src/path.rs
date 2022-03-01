use bevy_core::Name;
use bevy_reflect::TypeRegistry;
use std::any::TypeId;
use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

/// A named path through a hierarchy of entities.
///
/// This represents a String-like path taking the form of "root/a/b/c/d". When parsing,
/// this type will skip any preceding backslashes, so `////root//hips` is the same as
/// `root//hips`.
///
/// This type comes pre-split into individual levels, unlike a normal string.
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct EntityPath {
    parts: Vec<Name>,
}

impl EntityPath {
    const SEPERATOR: &'static str = "/";

    pub fn from_parts(parts: Vec<Name>) -> Self {
        Self { parts }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Name> {
        self.parts.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Name> {
        self.parts.iter_mut()
    }

    pub fn push(&mut self, part: impl Into<Name>) {
        self.parts.push(part.into())
    }

    pub fn pop(&mut self) -> Option<Name> {
        self.parts.pop()
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
        Ok(Self {
            parts: src
                .split(Self::SEPERATOR)
                .into_iter()
                .skip_while(|part| part.is_empty())
                .map(|part| Name::new(part.to_string()))
                .collect(),
        })
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
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct FieldPath {
    component_type_id: TypeId,
    parts: Vec<String>,
}

impl FieldPath {
    const SEPERATOR: &'static str = ".";

    pub fn parse(registry: &TypeRegistry, path: &str) -> Result<Self, ParsePathError> {
        let mut type_id = None;
        let mut parts = Vec::new();
        for (idx, part) in path.split(Self::SEPERATOR).enumerate() {
            if part.is_empty() {
                return Err(ParsePathError::ContainsEmptyField);
            }
            if part.contains(char::is_whitespace) {
                return Err(ParsePathError::FieldContainsWhitespace);
            }
            if idx == 0 {
                if let Some(registration) = registry.get_with_name(part) {
                    type_id = Some(registration.type_id());
                } else {
                    return Err(ParsePathError::InvalidComponentType);
                }
            }
            parts.push(part.to_string().into());
        }
        if type_id.is_some() {
            Ok(Self {
                component_type_id: type_id.unwrap(),
                parts,
            })
        } else {
            Err(ParsePathError::NoComponentName)
        }
    }

    pub fn component_type_id(&self) -> TypeId {
        self.component_type_id
    }

    pub fn component_name(&self) -> &str {
        self.parts[0].as_ref()
    }

    pub fn field_path(&self) -> String {
        self.parts[1..].join(".")
    }

    pub fn iter(&self) -> impl Iterator<Item = &String> {
        self.parts.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut String> {
        self.parts.iter_mut()
    }

    pub fn push(&mut self, part: impl Into<String>) {
        self.parts.push(part.into())
    }

    pub fn pop(&mut self) -> Option<String> {
        if self.parts.len() > 1 {
            self.parts.pop()
        } else {
            None
        }
    }
}

impl fmt::Display for FieldPath {
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

/// A full property path selecting a single field within a hierarchy of
/// entities. Comprised of a [`EntityPath`] followed by a [`FieldPath`].
/// Each part of the full path is accessible separately.
///
/// This represents a String-like path taking the form of "root/a/b/c/@droot.a.b.c.d".
/// Each part of the path is delimited by a "@".
#[derive(Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct PropertyPath {
    entity: EntityPath,
    field: FieldPath,
}

impl PropertyPath {
    const SEPERATOR: char = '@';

    pub fn parse(registry: &TypeRegistry, path: &str) -> Result<Self, ParsePathError> {
        if let Some((entity, field)) = path.split_once(Self::SEPERATOR) {
            Ok(Self {
                entity: EntityPath::from_str(entity).unwrap(),
                field: FieldPath::parse(registry, field)?,
            })
        } else {
            Err(ParsePathError::MissingDelimiter)
        }
    }

    /// Constructs a [`PropertyPath`] from it's consistituent parts.
    pub fn from_parts(entity: EntityPath, field: FieldPath) -> Self {
        Self { entity, field }
    }

    /// Splits the property path into it's constituent parts.
    pub fn into_parts(self) -> (EntityPath, FieldPath) {
        (self.entity, self.field)
    }

    /// Gets a immutable reference to the [`EntityPath`] in the full property path.
    pub fn entity(&self) -> &EntityPath {
        &self.entity
    }

    /// Gets mutable reference to the [`EntityPath`] in the full property path.
    pub fn entity_mut(&mut self) -> &EntityPath {
        &mut self.entity
    }

    /// Gets immutable reference to the [`FieldPath`] in the full property path.
    pub fn field(&self) -> &FieldPath {
        &self.field
    }

    /// Gets mutable reference to the [`FieldPath`] in the full property path.
    pub fn field_mut(&mut self) -> &FieldPath {
        &mut self.field
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParsePathError {
    MissingDelimiter,
    ContainsEmptyField,
    FieldContainsWhitespace,
    InvalidComponentType,
    NoComponentName,
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
        let vec: Vec<_> = path.iter().collect();
        assert_eq!(vec, vec!["a", "b", "c", "d", "e", "f", "", "g"]);
    }

    #[test]
    pub fn test_parse_entity_path_ignore_leading_backslash() {
        let path_str = "///a/b/c/d/e/f//g";
        let path = EntityPath::from_str(path_str).unwrap();
        let vec: Vec<_> = path.iter().collect();
        assert_eq!(vec, vec!["a", "b", "c", "d", "e", "f", "", "g"]);
    }

    #[test]
    pub fn test_parse_field_path() {
        let mut registry = TypeRegistry::default();
        registry.register::<Test>();
        let path_str = "bevy_prototype_animation::path::test::Test.b.c.d.e.f.g";
        let path = FieldPath::parse(&registry, path_str).unwrap();
        let vec: Vec<_> = path.iter().collect();
        assert_eq!(
            vec,
            vec![
                "bevy_prototype_animation::path::test::Test",
                "b",
                "c",
                "d",
                "e",
                "f",
                "g"
            ]
        );
    }

    #[test]
    pub fn test_parse_field_path_fails_on_empty_field() {
        let mut registry = TypeRegistry::default();
        registry.register::<Test>();
        let path_str = "bevy_prototype_animation::path::test::Test.b.c.d.e.f..g";
        let path = FieldPath::parse(&registry, path_str);
        assert_eq!(path, Err(ParsePathError::ContainsEmptyField));
    }

    #[test]
    pub fn test_parse_field_path_fails_on_whitespace() {
        let mut registry = TypeRegistry::default();
        registry.register::<Test>();
        let path_str = "bevy_prototype_animation::path::test::Test.b.c.d.e.f a.g";
        let path = FieldPath::parse(&registry, path_str);
        assert_eq!(path, Err(ParsePathError::FieldContainsWhitespace));
    }

    #[test]
    pub fn test_parse_field_path_invalid_typek() {
        let mut registry = TypeRegistry::default();
        let path_str = "bevy_prototype_animation::path::test::Test.b.c.d.e.f a.g";
        let path = FieldPath::parse(&registry, path_str);
        assert_eq!(path, Err(ParsePathError::InvalidComponentType));
    }

    #[test]
    pub fn test_parse_property_path() {
        let mut registry = TypeRegistry::default();
        registry.register::<Test>();
        let path_str = "a/b/c/d/e/f//g@bevy_prototype_animation::path::test::Test.b.c.d.e.f.g";
        let path = PropertyPath::parse(&registry, path_str).unwrap();
        let entity_vec: Vec<_> = path.entity().iter().collect();
        let field_vec: Vec<_> = path.field().iter().collect();
        assert_eq!(entity_vec, vec!["a", "b", "c", "d", "e", "f", "", "g"]);
        assert_eq!(
            field_vec,
            vec![
                "bevy_prototype_animation::path::test::Test",
                "b",
                "c",
                "d",
                "e",
                "f",
                "g"
            ]
        );
    }

    #[test]
    pub fn test_parse_property_path_works_with_empty_entity() {
        let mut registry = TypeRegistry::default();
        registry.register::<Test>();
        let path_str = "@bevy_prototype_animation::path::test::Test.b.c.d.e.f.g";
        let path = PropertyPath::parse(&registry, path_str).unwrap();
        let entity_vec: Vec<_> = path.entity().iter().collect();
        let field_vec: Vec<_> = path.field().iter().collect();
        assert!(entity_vec.is_empty());
        assert_eq!(
            field_vec,
            vec![
                "bevy_prototype_animation::path::test::Test",
                "b",
                "c",
                "d",
                "e",
                "f",
                "g"
            ]
        );
    }

    #[test]
    pub fn test_parse_property_path_fails_on_empty_field() {
        let mut registry = TypeRegistry::default();
        registry.register::<Test>();
        let path_str = "a/b/c/d/e/f//g@bevy_prototype_animation::path::test::Test.b.c.d.e.f..g";
        let path = PropertyPath::parse(&registry, path_str);
        assert_eq!(path, Err(ParsePathError::ContainsEmptyField));
    }

    #[test]
    pub fn test_parse_property_path_fails_on_whitespace() {
        let mut registry = TypeRegistry::default();
        registry.register::<Test>();
        let path_str = "a/b/c/d/e/f//g@bevy_prototype_animation::path::test::Test.b.c.d.e.f a.g";
        let path = PropertyPath::parse(&registry, path_str);
        assert_eq!(path, Err(ParsePathError::FieldContainsWhitespace));
    }
}
