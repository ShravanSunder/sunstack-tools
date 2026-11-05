# Resource Naming

Use one shared local stack and make every producer identify itself with stable, low-cardinality resource attributes.

## Stack Commands And URLs

```bash
AI_TOOLS_OBSERVABILITY_STACK_HELPER=~/dev/ai-tools/observability/observability-stack
AI_TOOLS_OBSERVABILITY_COLLECTOR_HEALTH_URL=http://127.0.0.1:13133/
OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318
OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf
```

Docker Compose service names:

```text
ai-tools-otel-collector
ai-tools-victoria-metrics
ai-tools-victoria-logs
ai-tools-victoria-traces
```

## Required Producer Resource Attributes

```text
service.name
service.version
dev.repo.hash
dev.worktree.hash
dev.branch.name
dev.runtime.flavor
dev.release.channel
```

`dev.repo.hash` and `dev.worktree.hash` are the primary grouping keys for shared local data. `dev.branch.name` is useful context, but it must not be a VictoriaLogs stream field.

Recommended runtime values:

```text
dev.runtime.flavor=debug|beta|stable|agent-vm|openclaw|smoke
dev.release.channel=local|beta|stable|managed|external
```

## Proof-Only Attributes

Use these for fresh verification runs, not as stream fields:

```text
agent.proof.marker
agent.proof.started_at
agent.proof.state_file
```

State files should use portable keys:

```text
AGENTSTUDIO_OBSERVABILITY_MARKER
AGENTSTUDIO_OBSERVABILITY_QUERY_START
AGENT_VM_OBSERVABILITY_MARKER
AGENT_VM_OBSERVABILITY_QUERY_START
```

## Forbidden Stream Fields

Do not promote these to stream labels:

```text
git.branch
dev.branch.name
agent.proof.marker
process.pid
run.id
workspace.id
repo.path
worktree.path
prompt
payload
token
secret
password
api_key
error.raw
raw.path
```
