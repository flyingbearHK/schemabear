# Topic: Harness Bootstrap

Lifecycle: active  
Confidence: confirmed  
Last verified: 2026-07-22  
Source of truth: `AGENTS.md`, `~/.pi/agent/AGENTS.md`, `~/.pi/agent/agents/`

## Facts

- pi loads `~/.pi/agent/AGENTS.md` then walks to repo `AGENTS.md`.
- Subagents are not built into pi core; enabled via `extensions/subagent` spawning isolated `pi` processes.
- Modes: single, parallel (≤8 tasks / 4 concurrent in extension; project policy caps 5 workers), chain with `{previous}`.
- Agent defs are markdown + YAML frontmatter in `~/.pi/agent/agents/` (user) and optionally `.pi/agents/` (project).
- Persistent memory is repo-local under `memory/`; chat is ephemeral.

## Decision

Use orchestrator/worker as the default multi-agent pattern: primary keeps authority; workers get bounded packets; reviewer is read-only and independent.

## Supersedes

- none
