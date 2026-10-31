---
name: ops-observability-stack
description: Use when starting, checking, using, debugging, or verifying the shared local OpenTelemetry and Victoria stack for AgentStudio, Agent VM, OpenClaw, beta apps, debug apps, or any repo that emits OTLP locally.
---

# Ops Observability Stack

The runnable stack source of truth is `~/dev/ai-tools/observability`. App repos are telemetry producers: they may own launch flags, markers, state files, and source-side scrubbing, but they must not own Docker Compose, Victoria service lifecycle, or generic Victoria query cookbooks.

When a change would touch both the shared stack and a producer, or the Separation Rule doesn't name the owner of a file you are about to change, name the owner of each file before editing ([one owner per truth](../principle-one-owner-per-truth/SKILL.md)).

Choose the smallest loop first:

- IF the work is stack lifecycle only, load `~/dev/ai-tools/observability/README.md`, follow its lifecycle commands, and return the result each command reported, or the gap that stopped it.
- IF the work touches producer requirements, load `references/producer-contract.md` and return the requirements that apply.
- IF the work touches resource labels, marker/state-file keys, or service names, load `references/resource-naming.md` and return the names and keys that apply.
- IF the work is AgentStudio debug or beta, load `references/agentstudio-loop.md` and follow that loop.
- IF the work is Agent VM, OpenClaw, or `shravan-claw-beta`, load `references/resource-naming.md` and return the names and keys that apply, then load `references/agent-vm-loop.md` and follow the loop for its ownership mode.
- IF the work needs Victoria proof or debugging queries, load `references/victoria-queries.md` and return the queries for the fresh marker.

When expected telemetry is missing or a stack check fails, before restarting the stack or editing a producer, find the first point on the path you observed failing (reachability from this shell, collector health, the endpoint and resource attributes the running process read, the state-file marker), each labeled seen or inferred ([read ground truth first](../principle-read-ground-truth-first/SKILL.md)).

Before a health check or telemetry query closes a proof claim, name the target it observed and what it did not cover: a health-only result identifies the stack or health endpoint, while a producer-emission result identifies the app, worktree and marker matched; for a claim that a canary is absent, show where the canary entered the producer path under that marker, since a canary the run never received proves nothing by its absence ([verify before accepting](../principle-verify-before-accepting/SKILL.md)). Collector health supports a health claim, not app emission. For a producer claim, use the selected query loop to match the producer, resource labels and fresh marker to the target; a stale sample from another worktree cannot establish current app emission. Query access failure or timeout leaves emission inconclusive.

## Operating Model

Use one shared local Victoria stack and separate producers with resource attributes:

```bash
~/dev/ai-tools/observability/observability-stack up
~/dev/ai-tools/observability/observability-stack status
~/dev/ai-tools/observability/observability-stack smoke
```

Explicit observability launchers are strict: if the collector is absent, they should fail with a clear `Run: mise run observability:up` or equivalent message. Ordinary app startup is fail-open and must not crash when the collector is down.

Never move the shared stack to `devfiles`, and do not create per-app Victoria Compose files unless the user explicitly asks for a separate managed stack.

## Separation Rule

Keep these responsibilities apart:

- `ai-tools/observability`: shared Docker Compose, collector config, retention, smoke checks, and lifecycle commands.
- This skill: agent-facing workflow, Victoria query recipes, producer schema, and debugging loops.
- App repos: minimal producer adapters, strict debug/beta launchers, state files, marker emitters, and app-specific smoke tests.
- Agent VM managed mode: deployment-owned Victoria stack for that runtime only.
- Agent VM external mode: local development points to this shared ai-tools stack.

## Guardrails

- Accept only loopback OTLP endpoints and health URLs for shared local mode.
- Use `dev.repo.hash` and `dev.worktree.hash` as grouping keys.
- Keep branch names searchable but out of VictoriaLogs stream fields.
- Do not export prompts, payloads, raw paths, raw errors, tokens, passwords, or API keys.
- Treat collector redaction as defense-in-depth; producer code must scrub first.
- Do not use stale logs as proof. Fresh runs need fresh markers and state files.

Complete when the requested operational loop has an observed result or an exact access or evidence gap: health-only proof names the observed stack or health endpoint, and producer proof names the app, worktree and marker matched. Keep each claim within that observed scope.
