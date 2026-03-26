# Skill: validate-milestone

Use this skill before claiming a change is complete.

Default ladder:
- `cargo fmt`
- `cargo check`
- targeted tests for touched areas
- broader `cargo test` when needed
- `cargo run -p viewer_app` when runtime behavior changed

State exactly what ran and what happened.
If blocked, do not claim completion.
check docs/FIELD_GUIDE.md when locating non-obvious functions, data paths, or ownership points
