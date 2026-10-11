//! Convenient facade for RELAX NG XML schema parsing and validation.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub use relaxng_model::{Name, Pattern, Schema};
pub use relaxng_syntax::{Span, SyntaxError, parse_file, parse_str};
pub use relaxng_validator::{
    SourceSpan, ValidationError, ValidationErrorKind, ValidationReport, Validator,
};

/// Parse a schema and validate an XML document with default limits.
pub fn validate(schema_xml: &str, document_xml: &str) -> Result<(), ValidationErrorKind> {
    Validator::new().validate(schema_xml, document_xml)
}
