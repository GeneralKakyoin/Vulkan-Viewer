# Skill: boundary-check

Use this skill before implementation and during review.

Check:
- crate ownership
- interface boundaries
- whether the change crosses architecture seams
- whether Firestorm is being used only as reference, not template
check docs/FIELD_GUIDE.md when locating non-obvious functions, data paths, or ownership points

If the change crosses boundaries or architecture, stop and escalate.
