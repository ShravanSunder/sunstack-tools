# Brief: a notification service and its delivery queue (two pages)

Audience: an engineer joining the notifications team. Write two linked pages: an overview of the service, and a deep dive into its delivery queue. One guided tour starts on the overview, visits the deep dive, and returns to the overview. Illustrative service, dated 2026-10-14.

## Page 1: the service overview

Opening summary: The notification service turns product events into emails, push messages and SMS. It accepts about 2.1 million events a day and delivers 99.4% within one minute.

Parts:

- **Event intake** — receives product events from the event bus; validates and de-duplicates them by event id.
- **Preference store** — each user's channels, quiet hours and unsubscribes (Postgres).
- **Composer** — turns an event and a template into a message per channel.
- **Delivery queue** — holds messages until a channel worker sends them; retries failures (details on page 2).
- **Channel workers** — one per channel: email (via a mail provider), push (via the mobile push services), SMS (via an SMS gateway).

Flow: Event intake → Composer (reads Preference store) → Delivery queue → Channel workers.

Key numbers (last 30 days): 2.1 M events per day; 99.4% delivered within 1 minute; 0.3% permanently failed; p95 compose time 38 ms.

Proposed, not built: a **digest batcher** between Composer and Delivery queue that groups low-priority messages into one daily email per user. Approved in design review on 2026-10-10; implementation not started. Show it as a proposal that readers can switch off to see the current system.

## Page 2: the delivery queue

Message states: queued → sending → sent; sending → retrying (on a temporary failure) → sending; retrying → dead (after the last attempt); sending → dead (on a permanent failure).

Retry policy: exponential backoff starting at 10 seconds, doubling, at most 6 attempts, with up to 20% jitter. Permanent failures (invalid address, unsubscribed) go straight to dead.

The change under review (#341) caps backoff at 15 minutes and moves the policy into its own file:

```diff
diff --git a/src/queue/retry.ts b/src/queue/retry.ts
--- a/src/queue/retry.ts
+++ b/src/queue/retry.ts
@@ -8,6 +8,7 @@ export function nextDelay(attempt: number): number {
   const base = 10_000 * 2 ** attempt;
-  return base + jitter(base);
+  const capped = Math.min(base, policy.maxDelayMs);
+  return capped + jitter(capped);
 }
diff --git a/src/queue/policy.ts b/src/queue/policy.ts
--- a/src/queue/policy.ts
+++ b/src/queue/policy.ts
@@ -1,3 +1,4 @@
 export const policy = {
   maxAttempts: 6,
+  maxDelayMs: 15 * 60_000,
 };
```

Notes on that change: in retry.ts the capped line is the fix (attempt 6 used to wait over 10 minutes; now never more than 15 minutes in total per step); in policy.ts the new setting is added.

Configuration: `QUEUE_MAX_ATTEMPTS` 6; `QUEUE_BASE_DELAY_MS` 10000; `QUEUE_MAX_DELAY_MS` 900000 (new); `QUEUE_WORKERS_PER_CHANNEL` 8.

Known issues:

- Dead messages are kept 14 days but nothing alerts on a spike in dead messages.
- Push retries can deliver duplicates when the push service times out after accepting a message.

## The tour

1. The overview's summary.
2. The flow of parts.
3. On the deep dive: the message states.
4. On the deep dive: the retry change.
5. Back on the overview: the proposed digest batcher.
