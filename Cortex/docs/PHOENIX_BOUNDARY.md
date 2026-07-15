# Phoenix integration boundary

Phoenix integration is not enabled by default. Cortex must not automatically ingest private Phoenix surfaces such as `SOUL.md`, `JOURNAL.md`, wake digests, private memory folders, raw conversations, relationship data, or credentials.

An eventual adapter must use an explicit allowlist limited to source code, schemas, public architecture documentation, and task-selected operational logs. Denied paths belong in a repository-local `sensitive_exclude_patterns` configuration list before bootstrap.

Example:

```json
{
  "sensitive_exclude_patterns": [
    "SOUL.md", "JOURNAL.md", "memory", "memories", "conversations", "wake-digests"
  ]
}
```

This is a privacy boundary, not an adapter or authorization grant.
