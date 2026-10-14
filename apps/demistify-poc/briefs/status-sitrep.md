# Brief: morning status of a release train

Audience: the engineering lead, reading at 8:30 before stand-up. They want to see in one glance what can merge, what is blocked and on whom, and what changes before the next check-in. Illustrative project ("payments-gateway"), dated 2026-10-14, status as of 08:18.

Opening summary: Nothing is ready to merge. Two changes need one more proof each; one change is blocked on an owner decision; the infrastructure stack is paused.

## Changes

### #212 Idempotent refunds (owner: Priya)

- Passed: build, unit tests (412), lint, type check.
- Needs verification: replay test against the sandbox processor. Window booked 08:40–09:00. The previous window was lost because the sandbox token expired.
- In progress: second review (Marco), expected by 10:00.
- Not ready until: replay proof passes and the second review approves.

### #215 Currency rounding (owner: Ken)

- Passed: build, unit tests (388), property tests (10,000 cases).
- Blocked: product must decide banker's rounding versus half-up for JPY and KRW. Asked on 2026-10-13; no answer yet.
- Ready to merge as soon as the decision lands, if the decision is banker's rounding (already implemented). Half-up needs about two hours of change.

### #219 Webhook retries (owner: Priya)

- Passed: build and unit tests (97).
- Needs verification: load test at 2× peak (booked 11:00).
- Risk: retry storm if the processor is down for more than 15 minutes; a circuit breaker is proposed but not built.

### Infrastructure stack (owner: Dana) — paused

- #201 Postgres 16 upgrade: done in staging, waiting for the change-freeze to end on 2026-10-16.
- #204 Connection pool resize: depends on #201.
- #207 Read replica for reporting: depends on #204; design approved.

## Checklist for the 10:30 check-in

- Replay proof for #212 (not done)
- Second review for #212 (not done)
- Rounding decision for #215 (not done)
- Load test booked for #219 (done)
- Change-freeze end date confirmed (done)

## Needs the lead

- Chase the rounding decision for #215 with product.
- Decide whether #219 can merge without the circuit breaker if the load test passes.
