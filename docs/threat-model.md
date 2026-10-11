# Threat model

Schema and document strings may be attacker-controlled. The implementation bounds recursion, avoids network/file resolution, and reports malformed XML and unsupported features. It does not provide a global byte budget or guarantee resistance to all parser resource attacks, so callers should enforce request-size and timeout limits.
