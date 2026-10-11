# Architecture

The workspace is split into a model crate, an XML syntax parser, a validator, a facade, and a small CLI. The parser produces a tree of model patterns; references are resolved while constructing the schema. The validator walks XML nodes with bounded recursive matching and collects source-located diagnostics.
