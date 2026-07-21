# Context Retrieval Policy

Project deltas only. Global standard (when using Codex layered repos): `~/.codex/standards/context-management.md`.

## Defaults

- Load bootstrap set from `context_manifest.yml` only.
- One topic shard per task unless cross-cutting.
- Prefer code/tests over narrative when they disagree; record the conflict.
- Freshness prompts (not hard fails): active 14d, topics 30d, stable 90d.

## Write Policy

| Change | Update |
|--------|--------|
| Resume point / blockers | `ACTIVE_CONTEXT.md` |
| Active work | `tasks/todo.md` |
| Durable how/why | `topics/*.md` or `decisions/*.md` |
| Behavior of agents/commands | root `AGENTS.md` |
| No state change | nothing |

## Anti-Bloat

- No chat transcripts in memory.
- No file listings that mirror `ls`.
- Archive superseded notes; don't delete evidence without `superseded_by`.
