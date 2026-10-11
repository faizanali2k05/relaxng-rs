# relaxng-rs

`relaxng-rs` is a layered Rust implementation of RELAX NG XML syntax parsing and document validation. It is designed for applications that need schema validation without binding to a vendor-specific XML stack.

## Quick start

```toml
[dependencies]
relaxng-rs = "0.1"
```

```rust
let schema = r#"<element xmlns="http://relaxng.org/ns/structure/1.0" name="greeting"><text/></element>"#;
relaxng_rs::validate(schema, "<greeting>Hello</greeting>")?;
# Ok::<(), relaxng_rs::ValidationErrorKind>(())
```

The workspace also provides `relaxng-syntax`, `relaxng-model`, `relaxng-validator`, and the `relaxng` command-line tool:

```text
relaxng check-schema schema.rng
relaxng validate schema.rng document.xml
```

## Supported surface

The XML syntax parser supports element, attribute, text, empty, notAllowed, data, value, choice, group, interleave, optional, oneOrMore, zeroOrMore, grammar/start/define, references, and namespace-aware names. The validator implements bounded recursive matching, common string/token/integer/boolean datatypes, source spans, and structured diagnostics.

Compact Syntax, external references, and include resolution are reported as explicit unsupported features in 0.1 rather than silently interpreted. XML entity expansion is delegated to the underlying safe XML parser; applications should impose input-size limits before parsing untrusted data.

## Security and limits

Validation has a configurable maximum nesting depth (256 by default). The parser does not fetch URLs or files referenced by schemas. See `docs/security.md` and `docs/threat-model.md`.

## Compatibility

The MSRV is Rust 1.88. Linux, macOS, and Windows are covered by CI. The 0.1 API follows semantic versioning, with additive changes preferred before 1.0. For full RELAX NG standard conformance or Compact Syntax today, use a mature external implementation and treat this crate as an embeddable XML-syntax subset.

## License

Licensed under either MIT or Apache-2.0 at your option.
