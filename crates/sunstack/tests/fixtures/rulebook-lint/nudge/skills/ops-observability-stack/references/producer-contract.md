# Producer Contract

App and service repos are OTLP producers. They do not own the shared collector, Victoria services, Docker Compose lifecycle, retention flags, or generic query recipes.

## Startup Modes

Ordinary startup:

- Must be fail-open when the collector is absent.
- Must not crash or block app startup because OTLP export failed.
- May continue writing local JSON logs if the app supports them.

Explicit observability launchers:

- Must check the shared collector first.
- Must fail fast when it is absent.
- Must tell the operator to start the stack.
- Must emit a fresh marker and state file.
- Must use OTLP-only or clearly documented OTLP-plus-local behavior.

## Scrub Boundary

Producer code scrubs first. The collector scrubs second.

Producers must not emit:

- raw prompts or model payloads
- tool output bodies
- raw filesystem paths
- raw errors that may contain secrets
- tokens, passwords, API keys, cookies, or auth headers
- raw UUIDs or high-cardinality product ids as stream fields

The collector deletes known sensitive attributes and redacts secret-like variants, but that is defense-in-depth. Do not rely on the collector as the first privacy boundary.

## Cardinality Rules

Use stable resource fields for separation:

```text
service.name
service.version
dev.repo.hash
dev.worktree.hash
dev.runtime.flavor
dev.release.channel
```

Keep branch names, markers, PIDs, run ids, and workspace ids out of stream fields. They can be searchable log/span attributes when they are scrubbed and bounded.

## Repo Helper Expectations

Thin repo helpers should do only repo-specific work:

- build or locate the debug/beta artifact
- check the collector health URL
- set OTLP env vars
- set producer resource attributes
- write marker/state files
- stay attached when the process lifetime matters

They should not start, stop, restart, or delete the shared stack.
