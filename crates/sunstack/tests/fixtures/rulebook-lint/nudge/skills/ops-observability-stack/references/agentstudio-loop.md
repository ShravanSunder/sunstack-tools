# AgentStudio Loop

AgentStudio is a producer. It should not own Docker Compose, Victoria service lifecycle, or generic Victoria query recipes.

## Start The Shared Stack

From any shell:

```bash
~/dev/ai-tools/observability/observability-stack up
~/dev/ai-tools/observability/observability-stack status
```

If the repo exposes a local alias such as `mise run observability:up`, that alias should delegate to the ai-tools helper.

## Debug App

Expected repo helper shape:

```bash
mise run run-debug-observability
```

The helper should:

- build the debug app with visible build output
- fail if the shared collector is not healthy
- set `OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4318`
- set `OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf`
- force the app's observability backend to OTLP
- set full trace/log tags for debug proof
- write `tmp/debug-observability/latest-observability.env`
- stay attached so the app keeps running

Do not start the stack inside the debug launcher. The operator or agent starts the stack explicitly first.

## Beta App

Expected repo helper shape:

```bash
mise run run-beta-observability
```

The helper should launch the newest local beta bundle or the requested beta bundle, set the same OTLP env, write `tmp/beta-observability/latest-observability.env`, and stay attached.

If collector health fails, the strict beta launcher should exit and tell the operator how to start the shared stack. Ordinary beta startup outside this helper remains fail-open.

## Verify A Fresh Run

Read the state file from another shell:

```bash
source tmp/debug-observability/latest-observability.env
```

Then use the Victoria query recipes with the marker from the state file. Verification means the current marker appears in Victoria and sensitive canaries are absent. Do not accept old logs, old screenshots, or unmarked records as proof.

For native UI checks, use PID targeting for the launched debug/beta process. Do not target by app name when another live AgentStudio may be running.
