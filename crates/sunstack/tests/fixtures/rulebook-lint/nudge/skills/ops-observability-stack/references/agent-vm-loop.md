# Agent VM And OpenClaw Loop

Agent VM has two legitimate ownership modes. Keep them separate.

## Managed Mode

Managed mode is deployment-owned by Agent VM. It may start and supervise its own collector/Victoria services as part of a managed runtime. Managed-mode docs can explain those services, but the names and lifecycle belong to that deployment, not to the shared ai-tools stack.

Managed mode should still use safe producer labels:

```text
service.name
service.version
dev.repo.hash
dev.worktree.hash
dev.runtime.flavor
dev.release.channel=managed
```

For any shared local or managed proof, explicitly require `dev.repo.hash` and `dev.worktree.hash`. Do not hide these behind a generic "resource attributes" phrase; the labels are how unrelated repos and worktrees stay separated in one Victoria stack.

## External Shared-Local Mode

External mode is for local development across unrelated repos. It points Agent VM, OpenClaw, and `shravan-claw-beta` at:

```bash
~/dev/ai-tools/observability/observability-stack
```

Expected local sequence:

```bash
~/dev/ai-tools/observability/observability-stack up
pnpm observability:start
```

`pnpm observability:start` should be strict: it checks the shared collector, sets OTLP env/resource attributes, starts the local process with visible logs, and writes a marker/state file.

External-mode producer docs must name the shared grouping labels:

```text
dev.repo.hash
dev.worktree.hash
```

Plain `pnpm start` should stay fail-open unless the repo explicitly documents a strict observability profile. It is not proof that data reached Victoria.

## Scrubbing Gate

Do not enable external OTLP export when sensitive-log redaction is disabled. Fail before startup rather than sending unsafe telemetry and relying on the collector to clean it up.

## What Not To Put In Agent VM External Docs

Do not copy generic Victoria query recipes into Agent VM external-mode docs. Link to this skill instead. Agent VM docs can own only:

- how to choose managed vs external mode
- how to start its local process
- which marker/state file it writes
- which resource attributes it sets
- what proof gate command or script is repo-specific
