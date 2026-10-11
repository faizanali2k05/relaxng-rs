//! Typed data structures shared by the RELAX NG parser and validator.

#![deny(unsafe_code)]
#![warn(missing_docs)]

/// A qualified XML name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Name {
    /// Namespace URI, if one was declared.
    pub namespace: Option<String>,
    /// Local name.
    pub local: String,
}

impl Name {
    /// Create an unqualified name.
    pub fn new(local: impl Into<String>) -> Self {
        Self {
            namespace: None,
            local: local.into(),
        }
    }
    /// Create a name in a namespace.
    pub fn in_namespace(namespace: impl Into<String>, local: impl Into<String>) -> Self {
        Self {
            namespace: Some(namespace.into()),
            local: local.into(),
        }
    }
}

/// A RELAX NG schema pattern.
#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    /// Matches no content.
    Empty,
    /// Always fails.
    NotAllowed,
    /// Matches text content.
    Text,
    /// Matches a named datatype value.
    Data {
        /// Datatype name such as `string`, `integer`, or `boolean`.
        datatype: String,
    },
    /// Matches an exact text value.
    Value {
        /// Exact lexical value.
        value: String,
    },
    /// Matches an element with the given name and content pattern.
    Element {
        /// Element name.
        name: Name,
        /// Content pattern.
        pattern: Box<Pattern>,
    },
    /// Matches an attribute with the given name and text value pattern.
    Attribute {
        /// Attribute name.
        name: Name,
        /// Attribute value pattern.
        pattern: Box<Pattern>,
    },
    /// Matches either branch.
    Choice(Vec<Pattern>),
    /// Matches adjacent content in order.
    Group(Vec<Pattern>),
    /// Matches content in any order.
    Interleave(Vec<Pattern>),
    /// Matches one or more occurrences.
    OneOrMore(Box<Pattern>),
    /// Matches zero or more occurrences.
    ZeroOrMore(Box<Pattern>),
    /// Matches zero or one occurrence.
    Optional(Box<Pattern>),
    /// A named reference resolved by the schema parser.
    Ref(String),
}

impl Pattern {
    /// Combine patterns as a group, simplifying an empty group.
    pub fn group(patterns: Vec<Pattern>) -> Self {
        match patterns.as_slice() {
            [] => Self::Empty,
            [single] => single.clone(),
            _ => Self::Group(patterns),
        }
    }
    /// Combine patterns as a choice, simplifying a single branch.
    pub fn choice(patterns: Vec<Pattern>) -> Self {
        match patterns.as_slice() {
            [] => Self::NotAllowed,
            [single] => single.clone(),
            _ => Self::Choice(patterns),
        }
    }
}

/// A parsed RELAX NG schema.
#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    /// Root pattern.
    pub root: Pattern,
    /// Named definitions available to references.
    pub definitions: std::collections::BTreeMap<String, Pattern>,
}

impl Schema {
    /// Construct a schema from a root pattern.
    pub fn new(root: Pattern) -> Self {
        Self {
            root,
            definitions: Default::default(),
        }
    }
}
