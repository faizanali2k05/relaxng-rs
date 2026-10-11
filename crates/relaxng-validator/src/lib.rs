//! XML validation against the RELAX NG model.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use relaxng_model::{Name, Pattern, Schema};
use relaxng_syntax::{SyntaxError, parse_str};
use thiserror::Error;

/// A source span in an XML document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSpan {
    /// One-based line.
    pub line: u32,
    /// One-based column.
    pub column: u32,
}

/// One validation diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    /// Human-readable explanation.
    pub message: String,
    /// Location of the offending XML node.
    pub span: Option<SourceSpan>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(span) = &self.span {
            write!(formatter, "{}:{}: {}", span.line, span.column, self.message)
        } else {
            formatter.write_str(&self.message)
        }
    }
}

/// A collection of validation diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationReport {
    /// All diagnostics collected for the document.
    pub errors: Vec<ValidationError>,
}

impl ValidationReport {
    /// Returns true when no validation errors were collected.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Errors that prevent a document from being validated.
#[derive(Debug, Error)]
pub enum ValidationErrorKind {
    /// The XML document is not well formed.
    #[error("XML parse error: {0}")]
    Xml(String),
    /// The schema source could not be parsed.
    #[error("schema error: {0}")]
    Schema(#[from] SyntaxError),
    /// The document violated the schema.
    #[error("document is invalid")]
    Invalid(ValidationReport),
}

/// A validator with explicit resource limits.
#[derive(Clone, Debug)]
pub struct Validator {
    /// Maximum XML nesting depth accepted.
    pub max_depth: usize,
}

impl Default for Validator {
    fn default() -> Self {
        Self { max_depth: 256 }
    }
}

impl Validator {
    /// Create a validator with default limits.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum element nesting depth.
    pub fn max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth.max(1);
        self
    }

    /// Validate an XML string against a parsed schema.
    pub fn validate_str(&self, schema: &Schema, xml: &str) -> Result<(), ValidationErrorKind> {
        let document = roxmltree::Document::parse(xml)
            .map_err(|error| ValidationErrorKind::Xml(error.to_string()))?;
        let root = document.root_element();
        let mut errors = Vec::new();
        if !match_element(&schema.root, root, 0, self.max_depth, &mut errors) && errors.is_empty() {
            errors.push(diagnostic(root, "document root does not match the schema"));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrorKind::Invalid(ValidationReport { errors }))
        }
    }

    /// Parse a RELAX NG XML schema and validate an XML document.
    pub fn validate(&self, schema_xml: &str, xml: &str) -> Result<(), ValidationErrorKind> {
        let schema = parse_str(schema_xml)?;
        self.validate_str(&schema, xml)
    }
}

fn match_element(
    pattern: &Pattern,
    node: roxmltree::Node<'_, '_>,
    depth: usize,
    max_depth: usize,
    errors: &mut Vec<ValidationError>,
) -> bool {
    if depth >= max_depth {
        errors.push(diagnostic(node, "maximum XML nesting depth exceeded"));
        return false;
    }
    match pattern {
        Pattern::Element { name, pattern } => {
            if !name_matches(name, node) {
                errors.push(diagnostic(
                    node,
                    &format!("expected element `{}`", name.local),
                ));
                return false;
            }
            validate_attributes(pattern, node, errors);
            let items: Vec<_> = node
                .children()
                .filter(|child| {
                    child.is_element()
                        || (child.is_text() && !child.text().unwrap_or_default().trim().is_empty())
                })
                .collect();
            match_sequence(pattern, &items, depth + 1, max_depth, errors, node)
        }
        _ => match_sequence(pattern, &[], depth, max_depth, errors, node),
    }
}

fn match_sequence(
    pattern: &Pattern,
    items: &[roxmltree::Node<'_, '_>],
    depth: usize,
    max_depth: usize,
    errors: &mut Vec<ValidationError>,
    context: roxmltree::Node<'_, '_>,
) -> bool {
    match pattern {
        Pattern::Empty | Pattern::Attribute { .. } => items.is_empty(),
        Pattern::NotAllowed => false,
        Pattern::Text => items.iter().all(|item| item.is_text()),
        Pattern::Data { datatype } => {
            items.len() == 1
                && items[0].is_text()
                && datatype_matches(datatype, items[0].text().unwrap_or_default())
        }
        Pattern::Value { value } => {
            items.len() == 1
                && items[0].is_text()
                && items[0].text().unwrap_or_default().trim() == value
        }
        Pattern::Element { .. } => {
            items.len() == 1
                && items[0].is_element()
                && match_element(pattern, items[0], depth, max_depth, errors)
        }
        Pattern::Choice(branches) => branches.iter().any(|branch| {
            match_sequence(branch, items, depth, max_depth, &mut Vec::new(), context)
        }),
        Pattern::Group(parts) => match_group(parts, items, depth, max_depth, errors, context),
        Pattern::Interleave(parts) => parts
            .iter()
            .all(|part| match_sequence(part, items, depth, max_depth, &mut Vec::new(), context)),
        Pattern::Optional(inner) => {
            items.is_empty() || match_sequence(inner, items, depth, max_depth, errors, context)
        }
        Pattern::OneOrMore(inner) => {
            !items.is_empty() && match_repeated(inner, items, depth, max_depth, errors, context, 1)
        }
        Pattern::ZeroOrMore(inner) => {
            items.is_empty() || match_repeated(inner, items, depth, max_depth, errors, context, 0)
        }
        Pattern::Ref(_) => false,
    }
}

fn match_group(
    parts: &[Pattern],
    items: &[roxmltree::Node<'_, '_>],
    depth: usize,
    max_depth: usize,
    errors: &mut Vec<ValidationError>,
    context: roxmltree::Node<'_, '_>,
) -> bool {
    if parts.is_empty() {
        return items.is_empty();
    }
    if parts.len() == 1 {
        return match_sequence(&parts[0], items, depth, max_depth, errors, context);
    }
    for split in 0..=items.len() {
        let mut local = Vec::new();
        if match_sequence(
            &parts[0],
            &items[..split],
            depth,
            max_depth,
            &mut local,
            context,
        ) && match_group(
            &parts[1..],
            &items[split..],
            depth,
            max_depth,
            &mut local,
            context,
        ) {
            return true;
        }
    }
    errors.push(diagnostic(
        context,
        "content does not match the required group",
    ));
    false
}

fn match_repeated(
    inner: &Pattern,
    items: &[roxmltree::Node<'_, '_>],
    depth: usize,
    max_depth: usize,
    errors: &mut Vec<ValidationError>,
    context: roxmltree::Node<'_, '_>,
    minimum: usize,
) -> bool {
    let mut matched = 0;
    for item in items {
        if match_sequence(
            inner,
            std::slice::from_ref(item),
            depth,
            max_depth,
            &mut Vec::new(),
            context,
        ) {
            matched += 1;
        } else {
            errors.push(diagnostic(*item, "repeated content does not match"));
            return false;
        }
    }
    matched >= minimum
}

fn validate_attributes(
    pattern: &Pattern,
    node: roxmltree::Node<'_, '_>,
    errors: &mut Vec<ValidationError>,
) {
    let mut attributes = Vec::new();
    collect_attributes(pattern, &mut attributes);
    for (name, inner) in attributes {
        match node
            .attributes()
            .find(|attribute| attribute_matches(name, *attribute))
        {
            Some(attribute) if datatype_matches_pattern(inner, attribute.value()) => {}
            Some(_attribute) => errors.push(diagnostic(
                node,
                &format!("attribute `{}` has an invalid value", name.local),
            )),
            None => errors.push(diagnostic(
                node,
                &format!("missing required attribute `{}`", name.local),
            )),
        }
    }
}

fn collect_attributes<'a>(pattern: &'a Pattern, result: &mut Vec<(&'a Name, &'a Pattern)>) {
    match pattern {
        Pattern::Attribute { name, pattern } => result.push((name, pattern)),
        Pattern::Group(parts) | Pattern::Interleave(parts) | Pattern::Choice(parts) => parts
            .iter()
            .for_each(|part| collect_attributes(part, result)),
        Pattern::Optional(inner) | Pattern::OneOrMore(inner) | Pattern::ZeroOrMore(inner) => {
            collect_attributes(inner, result)
        }
        _ => {}
    }
}

fn datatype_matches_pattern(pattern: &Pattern, value: &str) -> bool {
    match pattern {
        Pattern::Text => true,
        Pattern::Data { datatype } => datatype_matches(datatype, value),
        Pattern::Value { value: expected } => value.trim() == expected,
        Pattern::Choice(parts) => parts
            .iter()
            .any(|part| datatype_matches_pattern(part, value)),
        _ => true,
    }
}

fn datatype_matches(datatype: &str, value: &str) -> bool {
    match datatype {
        "string" | "token" => true,
        "integer" | "int" => value.trim().parse::<i64>().is_ok(),
        "boolean" => matches!(value.trim(), "true" | "false" | "1" | "0"),
        _ => true,
    }
}

fn name_matches(name: &Name, node: roxmltree::Node<'_, '_>) -> bool {
    name.local == node.tag_name().name()
        && name
            .namespace
            .as_deref()
            .map(|namespace| node.tag_name().namespace() == Some(namespace))
            .unwrap_or(true)
}
fn attribute_matches(name: &Name, attribute: roxmltree::Attribute<'_, '_>) -> bool {
    name.local == attribute.name()
        && name
            .namespace
            .as_deref()
            .map(|namespace| attribute.namespace() == Some(namespace))
            .unwrap_or(true)
}
fn diagnostic(node: roxmltree::Node<'_, '_>, message: &str) -> ValidationError {
    let position = node.document().text_pos_at(node.range().start);
    ValidationError {
        message: message.to_owned(),
        span: Some(SourceSpan {
            line: position.row,
            column: position.col,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_element_and_attribute() {
        let schema = parse_str(r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="book"><attribute name="id"><data type="integer"/></attribute><element name="title"><text/></element></element>"#).unwrap();
        let validator = Validator::new();
        assert!(
            validator
                .validate_str(&schema, r#"<book id="4"><title>Rust</title></book>"#)
                .is_ok()
        );
        assert!(
            validator
                .validate_str(&schema, r#"<book id="x"><title>Rust</title></book>"#)
                .is_err()
        );
    }
    #[test]
    fn rejects_deep_documents() {
        let schema = parse_str(r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="a"><zeroOrMore><element name="a"><empty/></element></zeroOrMore></element>"#).unwrap();
        assert!(
            Validator::new()
                .max_depth(1)
                .validate_str(&schema, "<a><a/></a>")
                .is_err()
        );
    }
}
