# Design decisions

- Unsupported standard features return typed errors rather than silently accepting a weaker interpretation.
- No schema include or external reference is fetched by the library; this prevents hidden network and filesystem access.
- The validator has an explicit depth limit to bound recursive schemas and hostile XML.
- The model is independent of XML parsing so callers can construct schemas programmatically.
