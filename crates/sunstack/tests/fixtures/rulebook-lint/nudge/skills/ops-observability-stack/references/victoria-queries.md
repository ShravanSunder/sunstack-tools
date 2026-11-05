# Victoria Queries

Use fresh markers from producer state files. Do not use old logs as proof.

Defaults:

```bash
LOGS_URL=http://127.0.0.1:9428
METRICS_URL=http://127.0.0.1:8428
TRACES_URL=http://127.0.0.1:10428
```

## VictoriaLogs

Find records for a marker:

```bash
curl --silent --show-error --max-time 5 \
  "$LOGS_URL/select/logsql/query" \
  --data-urlencode "query=agent.proof.marker:$MARKER"
```

Bound by service and worktree:

```bash
curl --silent --show-error --max-time 5 \
  "$LOGS_URL/select/logsql/query" \
  --data-urlencode "query=service.name:$SERVICE dev.worktree.hash:$WORKTREE_HASH agent.proof.marker:$MARKER"
```

Sensitive canary negative check:

```bash
curl --silent --show-error --max-time 5 \
  "$LOGS_URL/select/logsql/query" \
  --data-urlencode "query=$SECRET_CANARY"
```

The response must not contain the canary.

## VictoriaMetrics

Export samples for a metric:

```bash
curl --silent --show-error --max-time 5 \
  "$METRICS_URL/api/v1/export" \
  --data-urlencode 'match[]=your_metric_name'
```

Check label sets for a forbidden value:

```bash
curl --get --silent --show-error --max-time 5 \
  "$METRICS_URL/api/v1/series" \
  --data-urlencode 'match[]=your_metric_name'
```

The label-set response must not contain tokens, secrets, raw paths, prompts, or payload canaries.

## VictoriaTraces

Find spans by proof marker:

```bash
curl --silent --show-error --max-time 5 \
  "$TRACES_URL/select/logsql/query" \
  --data-urlencode "query=\"span_attr:agent.proof.marker\":\"$MARKER\""
```

Sensitive canary negative checks:

```bash
curl --silent --show-error --max-time 5 \
  "$TRACES_URL/select/logsql/query" \
  --data-urlencode "query=\"span_attr:token\":\"$SECRET_CANARY\""

curl --silent --show-error --max-time 5 \
  "$TRACES_URL/select/logsql/query" \
  --data-urlencode "query=\"resource_attr:secret.token\":\"$SECRET_CANARY\""
```

Both responses must omit the canary.

## Interpretation

Positive proof needs a current marker and the expected service/resource labels. Negative proof needs the exact canary absent from logs, traces, and metric label sets. If a query times out, report it as inconclusive rather than green.
