use std::fmt;
use std::num::ParseIntError;

use bevy_reflect::{Reflect, ReflectMut, ReflectRef};
use thiserror::Error;

/// An error returned from a failed path string query.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum ReflectPathError<'a> {
    #[error("expected an identifier at the given index")]
    ExpectedIdent { index: usize },
    #[error("the current struct doesn't have a field with the given name")]
    InvalidField { index: usize, field: &'a str },
    #[error("the current tuple struct doesn't have a field with the given index")]
    InvalidTupleStructIndex {
        index: usize,
        tuple_struct_index: usize,
    },
    #[error("the current list doesn't have a value at the given index")]
    InvalidListIndex { index: usize, list_index: usize },
    #[error("encountered an unexpected token")]
    UnexpectedToken { index: usize, token: &'a str },
    #[error("expected a token, but it wasn't there.")]
    ExpectedToken { index: usize, token: &'a str },
    #[error("expected a struct, but found a different reflect value")]
    ExpectedStruct { index: usize },
    #[error("expected a list, but found a different reflect value")]
    ExpectedList { index: usize },
    #[error("failed to parse a usize")]
    IndexParseError(#[from] ParseIntError),
    #[error("failed to downcast to the path result to the given type")]
    InvalidDowncast,
}

/// A path to a field within a type. Can be used like [`Reflect::GetPath`] functions to get
/// references to the inner fields of a type.
///
/// [`GetPath`]: bevy_reflect::GetPath
#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct FieldPath(Box<[(Access, usize)]>);

impl FieldPath {
    /// Parses a [`FieldPath`] from a string. For the exact format, see [`GetPath`].
    /// Returns an error if the string does not represent a valid path to a field.
    pub fn parse(string: &str) -> Result<Self, ReflectPathError<'_>> {
        let mut parts = Vec::new();
        for (access, idx) in PathParser::new(string) {
            parts.push((access?.to_owned(), idx));
        }
        Ok(Self(parts.into_boxed_slice()))
    }

    /// Gets a read-only reference of given field.
    /// Returns an error if the path is invalid for the provided type.
    pub fn field<'r, 'p>(
        &'p self,
        root: &'r dyn Reflect,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
        let mut current: &dyn Reflect = root;
        for (access, current_index) in self.0.iter() {
            current = access.to_ref().read_field(current, *current_index)?;
        }
        Ok(current)
    }

    /// Gets a mutable reference of given field.
    /// Returns an error if the path is invalid for the provided type.
    pub fn field_mut<'r, 'p>(
        &'p self,
        root: &'r mut dyn Reflect,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        let mut current: &mut dyn Reflect = root;
        for (access, current_index) in self.0.iter() {
            current = access.to_ref().read_field_mut(current, *current_index)?;
        }
        Ok(current)
    }
}

impl fmt::Display for FieldPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, (access, _)) in self.0.iter().enumerate() {
            if idx != 0 {
                f.write_str(".")?;
            }
            match access {
                Access::Field(field) => {
                    f.write_str(field)?;
                }
                Access::TupleIndex(idx) => {
                    idx.fmt(f)?;
                }
                Access::ListIndex(idx) => {
                    f.write_str("[")?;
                    idx.fmt(f)?;
                    f.write_str("]")?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Access {
    Field(String),
    TupleIndex(usize),
    ListIndex(usize),
}

impl Access {
    fn to_ref(&self) -> AccessRef<'_> {
        match self {
            Self::Field(value) => AccessRef::Field(value),
            Self::TupleIndex(value) => AccessRef::TupleIndex(*value),
            Self::ListIndex(value) => AccessRef::ListIndex(*value),
        }
    }
}

#[derive(Debug)]
enum AccessRef<'a> {
    Field(&'a str),
    TupleIndex(usize),
    ListIndex(usize),
}

impl<'a> AccessRef<'a> {
    fn to_owned(&self) -> Access {
        match self {
            Self::Field(value) => Access::Field(value.to_string()),
            Self::TupleIndex(value) => Access::TupleIndex(*value),
            Self::ListIndex(value) => Access::ListIndex(*value),
        }
    }

    fn read_field<'r>(
        &self,
        current: &'r dyn Reflect,
        current_index: usize,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'a>> {
        match (self, current.reflect_ref()) {
            (Self::Field(field), ReflectRef::Struct(reflect_struct)) => reflect_struct
                .field(field)
                .ok_or(ReflectPathError::InvalidField {
                    index: current_index,
                    field,
                }),
            (Self::TupleIndex(tuple_index), ReflectRef::TupleStruct(reflect_struct)) => {
                reflect_struct.field(*tuple_index).ok_or(
                    ReflectPathError::InvalidTupleStructIndex {
                        index: current_index,
                        tuple_struct_index: *tuple_index,
                    },
                )
            }
            (Self::ListIndex(list_index), ReflectRef::List(reflect_list)) => reflect_list
                .get(*list_index)
                .ok_or(ReflectPathError::InvalidListIndex {
                    index: current_index,
                    list_index: *list_index,
                }),
            (Self::ListIndex(_), _) => Err(ReflectPathError::ExpectedList {
                index: current_index,
            }),
            _ => Err(ReflectPathError::ExpectedStruct {
                index: current_index,
            }),
        }
    }

    fn read_field_mut<'r>(
        &self,
        current: &'r mut dyn Reflect,
        current_index: usize,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'a>> {
        match (self, current.reflect_mut()) {
            (Self::Field(field), ReflectMut::Struct(reflect_struct)) => reflect_struct
                .field_mut(field)
                .ok_or(ReflectPathError::InvalidField {
                    index: current_index,
                    field,
                }),
            (Self::TupleIndex(tuple_index), ReflectMut::TupleStruct(reflect_struct)) => {
                reflect_struct.field_mut(*tuple_index).ok_or(
                    ReflectPathError::InvalidTupleStructIndex {
                        index: current_index,
                        tuple_struct_index: *tuple_index,
                    },
                )
            }
            (Self::ListIndex(list_index), ReflectMut::List(reflect_list)) => reflect_list
                .get_mut(*list_index)
                .ok_or(ReflectPathError::InvalidListIndex {
                    index: current_index,
                    list_index: *list_index,
                }),
            (Self::ListIndex(_), _) => Err(ReflectPathError::ExpectedList {
                index: current_index,
            }),
            _ => Err(ReflectPathError::ExpectedStruct {
                index: current_index,
            }),
        }
    }
}

struct PathParser<'a> {
    path: &'a str,
    index: usize,
}

impl<'a> PathParser<'a> {
    fn new(path: &'a str) -> Self {
        Self { path, index: 0 }
    }

    fn next_token(&mut self) -> Option<Token<'a>> {
        if self.index >= self.path.len() {
            return None;
        }

        match self.path[self.index..].chars().next().unwrap() {
            '.' => {
                self.index += 1;
                return Some(Token::Dot);
            }
            '[' => {
                self.index += 1;
                return Some(Token::OpenBracket);
            }
            ']' => {
                self.index += 1;
                return Some(Token::CloseBracket);
            }
            _ => {}
        }

        // we can assume we are parsing an ident now
        for (char_index, character) in self.path[self.index..].chars().enumerate() {
            match character {
                '.' | '[' | ']' => {
                    let ident = Token::Ident(&self.path[self.index..self.index + char_index]);
                    self.index += char_index;
                    return Some(ident);
                }
                _ => {}
            }
        }
        let ident = Token::Ident(&self.path[self.index..]);
        self.index = self.path.len();
        Some(ident)
    }

    fn token_to_access(&mut self, token: Token<'a>) -> Result<AccessRef<'a>, ReflectPathError<'a>> {
        let current_index = self.index;
        match token {
            Token::Dot => {
                if let Some(Token::Ident(value)) = self.next_token() {
                    value
                        .parse::<usize>()
                        .map(AccessRef::TupleIndex)
                        .or(Ok(AccessRef::Field(value)))
                } else {
                    Err(ReflectPathError::ExpectedIdent {
                        index: current_index,
                    })
                }
            }
            Token::OpenBracket => {
                let access = if let Some(Token::Ident(value)) = self.next_token() {
                    AccessRef::ListIndex(value.parse::<usize>()?)
                } else {
                    return Err(ReflectPathError::ExpectedIdent {
                        index: current_index,
                    });
                };

                if !matches!(self.next_token(), Some(Token::CloseBracket)) {
                    return Err(ReflectPathError::ExpectedToken {
                        index: current_index,
                        token: "]",
                    });
                }

                Ok(access)
            }
            Token::CloseBracket => Err(ReflectPathError::UnexpectedToken {
                index: current_index,
                token: "]",
            }),
            Token::Ident(value) => value
                .parse::<usize>()
                .map(AccessRef::TupleIndex)
                .or(Ok(AccessRef::Field(value))),
        }
    }
}

impl<'a> Iterator for PathParser<'a> {
    type Item = (Result<AccessRef<'a>, ReflectPathError<'a>>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token()?;
        let index = self.index;
        Some((self.token_to_access(token), index))
    }
}

enum Token<'a> {
    Dot,
    OpenBracket,
    CloseBracket,
    Ident(&'a str),
}
