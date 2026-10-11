# Security

Callers should bound input bytes before parsing. The parser performs no network access and does not resolve external references. The validator defaults to a 256-level element depth limit and returns errors for deeper documents. The implementation contains no unsafe Rust. XML entity and DTD policy is delegated to the underlying parser; applications handling hostile XML should reject DTD-heavy inputs before validation.
