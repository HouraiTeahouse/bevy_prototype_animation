use serde::{Deserialize, Serialize};
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
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct EntityPath {
    parts: Vec<Cow<'static, str>>,
}

impl EntityPath {
    const SEPERATOR: &'static str = "/";

    pub fn iter(&self) -> impl Iterator<Item = &'_ str> {
        self.parts.iter().map(|part| part.as_ref())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &'_ mut Cow<'static, str>> {
        self.parts.iter_mut()
    }

    pub fn push(&mut self, part: impl Into<Cow<'static, str>>) {
        self.parts.push(part.into())
    }

    pub fn pop(&mut self) -> Option<Cow<'static, str>> {
        self.parts.pop()
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
                .map(|part| part.to_string().into())
                .collect(),
        })
    }
}

impl fmt::Display for EntityPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.parts.join(Self::SEPERATOR))
    }
}

/// A named field path through a component type.
///
/// This represents a String-like path taking the form of "root.a.b.c.d".
///
/// This type comes pre-split into individual levels, unlike a normal string.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct FieldPath {
    parts: Vec<Cow<'static, str>>,
}

impl FieldPath {
    const SEPERATOR: &'static str = ".";

    pub fn iter(&self) -> impl Iterator<Item = &'_ str> {
        self.parts.iter().map(|part| part.as_ref())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &'_ mut Cow<'static, str>> {
        self.parts.iter_mut()
    }

    pub fn push(&mut self, part: impl Into<Cow<'static, str>>) {
        self.parts.push(part.into())
    }

    pub fn pop(&mut self) -> Option<Cow<'static, str>> {
        self.parts.pop()
    }
}

impl FromStr for FieldPath {
    type Err = ParsePathError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let mut parts = Vec::new();
        for part in src.split(Self::SEPERATOR) {
            if part.is_empty() {
                return Err(ParsePathError::ContainsEmptyField);
            }
            if part.contains(char::is_whitespace) {
                return Err(ParsePathError::FieldContainsWhitespace);
            }
            parts.push(part.to_string().into());
        }
        Ok(Self { parts })
    }
}

impl fmt::Display for FieldPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.parts.join(Self::SEPERATOR))
    }
}

/// A full property path selecting a single field within a hierarchy of
/// entities. Comprised of a [`EntityPath`] followed by a [`FieldPath`].
/// Each part of the full path is accessible separately.
///
/// This represents a String-like path taking the form of "root/a/b/c/@droot.a.b.c.d".
/// Each part of the path is delimited by a "@".
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct PropertyPath {
    entity: EntityPath,
    field: FieldPath,
}

impl PropertyPath {
    const SEPERATOR: char = '@';

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

impl FromStr for PropertyPath {
    type Err = ParsePathError;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        if let Some((entity, field)) = src.split_once(Self::SEPERATOR) {
            Ok(Self {
                entity: EntityPath::from_str(entity).unwrap(),
                field: FieldPath::from_str(field)?,
            })
        } else {
            Err(ParsePathError::MissingDelimiter)
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParsePathError {
    MissingDelimiter,
    ContainsEmptyField,
    FieldContainsWhitespace,
}

#[cfg(test)]
mod test {
    use super::*;

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
        let path_str = "a.b.c.d.e.f.g";
        let path = FieldPath::from_str(path_str).unwrap();
        let vec: Vec<_> = path.iter().collect();
        assert_eq!(vec, vec!["a", "b", "c", "d", "e", "f", "g"]);
    }

    #[test]
    pub fn test_parse_field_path_fails_on_empty_field() {
        let path_str = "a.b.c.d.e.f..g";
        let path = FieldPath::from_str(path_str);
        assert_eq!(path, Err(ParsePathError::ContainsEmptyField));
    }

    #[test]
    pub fn test_parse_field_path_fails_on_whitespace() {
        let path_str = "a.b.c.d.e.f a.g";
        let path = FieldPath::from_str(path_str);
        assert_eq!(path, Err(ParsePathError::FieldContainsWhitespace));
    }

    #[test]
    pub fn test_parse_property_path() {
        let path_str = "a/b/c/d/e/f//g@a.b.c.d.e.f.g";
        let path = PropertyPath::from_str(path_str).unwrap();
        let entity_vec: Vec<_> = path.entity().iter().collect();
        let field_vec: Vec<_> = path.field().iter().collect();
        assert_eq!(entity_vec, vec!["a", "b", "c", "d", "e", "f", "", "g"]);
        assert_eq!(field_vec, vec!["a", "b", "c", "d", "e", "f", "g"]);
    }

    #[test]
    pub fn test_parse_property_path_works_with_empty_entity() {
        let path_str = "@a.b.c.d.e.f.g";
        let path = PropertyPath::from_str(path_str).unwrap();
        let entity_vec: Vec<_> = path.entity().iter().collect();
        let field_vec: Vec<_> = path.field().iter().collect();
        assert!(entity_vec.is_empty());
        assert_eq!(field_vec, vec!["a", "b", "c", "d", "e", "f", "g"]);
    }

    #[test]
    pub fn test_parse_property_path_fails_on_empty_field() {
        let path_str = "a/b/c/d/e/f//g@a.b.c.d.e.f..g";
        let path = PropertyPath::from_str(path_str);
        assert_eq!(path, Err(ParsePathError::ContainsEmptyField));
    }

    #[test]
    pub fn test_parse_property_path_fails_on_whitespace() {
        let path_str = "a/b/c/d/e/f//g@a.b.c.d.e.f a.g";
        let path = PropertyPath::from_str(path_str);
        assert_eq!(path, Err(ParsePathError::FieldContainsWhitespace));
    }
}