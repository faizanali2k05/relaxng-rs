# Performance

The validator is a straightforward recursive matcher intended for configuration and document validation. It allocates diagnostics only for failing branches and uses a caller-configurable depth limit. No broad throughput claim is made without application-specific benchmarks; measure representative schemas and documents before selecting it for high-volume workloads.
