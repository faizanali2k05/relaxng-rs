//! Parser for the RELAX NG XML syntax.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use relaxng_model::{Name, Pattern, Schema};
use thiserror::Error;

/// A source position in a schema document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// One-based line.
    pub line: u32,
    /// One-based column.
    pub column: u32,
}

/// Schema parsing failures.
#[derive(Debug, Error)]
pub enum SyntaxError {
    /// XML was not well formed.
    #[error("XML parse error: {0}")]
    Xml(String),
    /// The schema uses a feature not implemented by this release.
    #[error("unsupported RELAX NG feature `{feature}` at line {span_line}, column {span_column}")]
    Unsupported {
        /// Unsupported construct name.
        feature: String,
        /// One-based line.
        span_line: u32,
        /// One-based column.
        span_column: u32,
    },
    /// A required schema attribute or child is absent.
    #[error("invalid RELAX NG schema at line {span_line}, column {span_column}: {message}")]
    Invalid {
        /// Explanation.
        message: String,
        /// One-based line.
        span_line: u32,
        /// One-based column.
        span_column: u32,
    },
    /// File access failure.
    #[error("could not read schema: {0}")]
    Io(#[from] std::io::Error),
}

/// Parse a RELAX NG XML syntax document.
pub fn parse_str(source: &str) -> Result<Schema, SyntaxError> {
    let document =
        roxmltree::Document::parse(source).map_err(|error| SyntaxError::Xml(error.to_string()))?;
    let root = document.root_element();
    let mut definitions = BTreeMap::new();
    let pattern = if root.tag_name().name() == "grammar" {
        for child in root
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "define")
        {
            let name = required_attr(child, "name")?;
            let pattern = parse_container(child, &mut definitions)?;
            definitions.insert(name, pattern);
        }
        let start = root
            .children()
            .find(|node| node.is_element() && node.tag_name().name() == "start")
            .ok_or_else(|| invalid(root, "grammar has no start pattern"))?;
        parse_container(start, &mut definitions)?
    } else {
        parse_pattern(root, &mut definitions)?
    };
    Ok(Schema {
        root: resolve_refs(pattern, &definitions),
        definitions,
    })
}

/// Parse a schema file from disk.
pub fn parse_file(path: impl AsRef<Path>) -> Result<Schema, SyntaxError> {
    parse_str(&fs::read_to_string(path)?)
}

fn parse_container(
    node: roxmltree::Node<'_, '_>,
    definitions: &mut BTreeMap<String, Pattern>,
) -> Result<Pattern, SyntaxError> {
    let children: Result<Vec<_>, _> = node
        .children()
        .filter(|child| child.is_element())
        .map(|child| parse_pattern(child, definitions))
        .collect();
    Ok(Pattern::group(children?))
}

fn parse_pattern(
    node: roxmltree::Node<'_, '_>,
    definitions: &mut BTreeMap<String, Pattern>,
) -> Result<Pattern, SyntaxError> {
    let kind = node.tag_name().name();
    match kind {
        "empty" => Ok(Pattern::Empty),
        "notAllowed" => Ok(Pattern::NotAllowed),
        "text" => Ok(Pattern::Text),
        "data" => Ok(Pattern::Data {
            datatype: node.attribute("type").unwrap_or("string").to_owned(),
        }),
        "value" => Ok(Pattern::Value {
            value: node.text().unwrap_or_default().trim().to_owned(),
        }),
        "ref" => Ok(Pattern::Ref(required_attr(node, "name")?)),
        "element" => {
            let name = parse_name(node)?;
            Ok(Pattern::Element {
                name,
                pattern: Box::new(parse_container(node, definitions)?),
            })
        }
        "attribute" => {
            let name = parse_name(node)?;
            Ok(Pattern::Attribute {
                name,
                pattern: Box::new(parse_container(node, definitions)?),
            })
        }
        "choice" => Ok(Pattern::choice(
            node.children()
                .filter(|child| child.is_element())
                .map(|child| parse_pattern(child, definitions))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        "group" => Ok(Pattern::group(
            node.children()
                .filter(|child| child.is_element())
                .map(|child| parse_pattern(child, definitions))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        "interleave" => Ok(Pattern::Interleave(
            node.children()
                .filter(|child| child.is_element())
                .map(|child| parse_pattern(child, definitions))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        "oneOrMore" => Ok(Pattern::OneOrMore(Box::new(parse_container(
            node,
            definitions,
        )?))),
        "zeroOrMore" => Ok(Pattern::ZeroOrMore(Box::new(parse_container(
            node,
            definitions,
        )?))),
        "optional" => Ok(Pattern::Optional(Box::new(parse_container(
            node,
            definitions,
        )?))),
        "define" | "start" | "grammar" => parse_container(node, definitions),
        "include" | "externalRef" => Err(unsupported(node, kind)),
        _ => Err(unsupported(node, kind)),
    }
}

fn parse_name(node: roxmltree::Node<'_, '_>) -> Result<Name, SyntaxError> {
    if let Some(value) = node.attribute("name") {
        return Ok(Name::new(value));
    }
    let child = node
        .children()
        .find(|child| child.is_element() && child.tag_name().name() == "name")
        .ok_or_else(|| invalid(node, "element or attribute requires a name"))?;
    Ok(match child.attribute("ns") {
        Some(namespace) => Name::in_namespace(namespace, child.text().unwrap_or_default().trim()),
        None => Name::new(child.text().unwrap_or_default().trim()),
    })
}

fn required_attr(node: roxmltree::Node<'_, '_>, name: &str) -> Result<String, SyntaxError> {
    node.attribute(name)
        .map(str::to_owned)
        .ok_or_else(|| invalid(node, &format!("missing `{name}` attribute")))
}

fn invalid(node: roxmltree::Node<'_, '_>, message: &str) -> SyntaxError {
    SyntaxError::Invalid {
        message: message.to_owned(),
        span_line: node.document().text_pos_at(node.range().start).row,
        span_column: node.document().text_pos_at(node.range().start).col,
    }
}

fn unsupported(node: roxmltree::Node<'_, '_>, feature: &str) -> SyntaxError {
    SyntaxError::Unsupported {
        feature: feature.to_owned(),
        span_line: node.document().text_pos_at(node.range().start).row,
        span_column: node.document().text_pos_at(node.range().start).col,
    }
}

fn resolve_refs(pattern: Pattern, definitions: &BTreeMap<String, Pattern>) -> Pattern {
    match pattern {
        Pattern::Ref(name) => definitions
            .get(&name)
            .cloned()
            .map(|value| resolve_refs(value, definitions))
            .unwrap_or(Pattern::NotAllowed),
        Pattern::Element { name, pattern } => Pattern::Element {
            name,
            pattern: Box::new(resolve_refs(*pattern, definitions)),
        },
        Pattern::Attribute { name, pattern } => Pattern::Attribute {
            name,
            pattern: Box::new(resolve_refs(*pattern, definitions)),
        },
        Pattern::Choice(values) => Pattern::Choice(
            values
                .into_iter()
                .map(|value| resolve_refs(value, definitions))
                .collect(),
        ),
        Pattern::Group(values) => Pattern::Group(
            values
                .into_iter()
                .map(|value| resolve_refs(value, definitions))
                .collect(),
        ),
        Pattern::Interleave(values) => Pattern::Interleave(
            values
                .into_iter()
                .map(|value| resolve_refs(value, definitions))
                .collect(),
        ),
        Pattern::OneOrMore(value) => {
            Pattern::OneOrMore(Box::new(resolve_refs(*value, definitions)))
        }
        Pattern::ZeroOrMore(value) => {
            Pattern::ZeroOrMore(Box::new(resolve_refs(*value, definitions)))
        }
        Pattern::Optional(value) => Pattern::Optional(Box::new(resolve_refs(*value, definitions))),
        value => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_namespace_and_group() {
        let schema = parse_str(r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="book"><element name="title"><text/></element></element>"#).unwrap();
        assert!(matches!(schema.root, Pattern::Element { .. }));
    }

    #[test]
    fn rejects_external_refs_explicitly() {
        let error = parse_str(r#"<externalRef href="other.rng"/>"#).unwrap_err();
        assert!(error.to_string().contains("externalRef"));
    }
}
