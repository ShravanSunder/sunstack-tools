# Executable phase 2 proof

Historical observations below retain the severity vocabulary and commands of their original target. Current severity values are `error` and `warn`; these records are not new runner proof.

Accepted native base: S3a `37f4034b3bcd526ca0d6d6b11fdc3c922eca8585`, merged
into the CLI through `7c64c1e5dd72b83dbec6a3da89e3b797e11af678`.
Oracle: Specification C14/V2/V13/V14/V29/V30 at `34bee48`, plus the adopted
8 MiB, readiness and optional-ceiling corrections `92d6bc6`, `8ac4133`,
`a32d32f`. Execution trace: Router root
`private-coordination-record`.

The CLI now exposes all four operations. Native decoders own admission and
diagnostics; native libraries own selection, scheduling, provider accounting,
verdicts and outcome variants. The CLI owns framing, Unix signals, environment
credential injection, serialization and exits. No native API, shape shim,
runtime fault flag or production stand-in was added. The milestone 2
`change_selections` surface remains intentionally absent from the milestone 1
native outcomes, per Lead disposition recorded in the phase 1 account.

## Real process scenarios

The permanent process harness spawns the actual built executable through the
unchanged absolute signing runner, with explicit CWD, stdin/stdout/stderr and a
joined child. Controlled HTTP replaces only the remote provider's replies.
It observes actual encoded request bodies and connection counts. The common
response checker validates literal exits, exactly one JSON response, bounded
static stderr, bounded diagnostics and absence of sentinel received values.

- `process_rules`: prepare_rules/prepared/0 with actual selected evidence;
  working HTTP positive control followed by preparation with zero additional
  calls and zero model_requests; complete_report/0 with an actual failed block
  verdict; native invalid/2 for both operations; absent valid U999 selection
  produces prepared_with_failures/3 and incomplete_report/3 with no dispatch;
  provider rejection/invalid answer produces incomplete_report/3 without a
  fabricated judgment.
- `process_rule_cancellation`: consumed SIGTERM before valid stdin produces
  cancelled/4 with before_preparation_admission or before_report_admission and
  zero HTTP. After report admission, max concurrency 1 makes receipt of the
  second full request a causal witness that the first native task completed.
  SIGTERM while that second request is held preserves the first judgment and
  receipt, cancels the second without retry despite retry_count=3, and leaves
  the third not_started. Two observed HTTP requests and files_read=2 reject
  cancellation that silently starts the third owner.
- `process_journeys`: both reviewed datasets prepare with zero dispatch;
  router preparation is not_evaluated, then complete evaluation retains the
  three intended rule IDs, severities, informational-answer references and
  targets. Router uncertainty/no-route gives complete/0 with no occurrences.
  Router provider failure gives incomplete/3 and evaluation_failed occurrences
  without judgments. Both comparison budgets preserve the exact occurrence
  keys, targets, judgments and question-owner multiplicities.
- The 12 phase 1 process cases continue to run, including both signal kinds,
  bounded stdin without EOF, evidence worker settlement, native judgment
  diagnostics, finite attempts and cursor/CWD behavior.

The focused native-enum serializer test constructs both PreparationFailed
variants, passes them through the real CLI ProgramResponse/exit path, and
checks unchanged diagnostics and measurements. V2 claim for preparation_failed:
**wire tag and exit mapping proven; no reachable public stdin input found, per
source-derived judgment (S3a 7302); runtime reachability not demonstrated**.
The Lead accepted this boundary in direct push
`private-coordination-record`. Native preparation returns this variant
only for private question-plan invariant failures; evaluation returns it for
scheduler invariant/worker-fatal failures. Public refusals use the real-process
exit 3 cases above.

The initial new-operation test failed because prepare_rules was unsupported
(`target/cli-phase2-rules-red.log`). After wiring it passed. Most newly added
cases first ran green; no pre-fix defect is claimed for those additions.
Post-green sensitivity deliberately forced evaluate_rules exits to 0. The
public selection-failure scenario rejected that fault (exit 0 instead of 3),
one failed, nextest exit 100, `target/cli-phase2-exit-sensitivity.log`. The fault
was restored before final gates.

## Journey inputs and independent checks

All ten reviewed example files remain unchanged. Skill lint selects U001/U002/U003
from the actual SKILL.md bytes, and checks source path and byte spans. Handoff
retains the reviewed hook origin and typed commission. It uses the accepted S2
public Codex composition, with explicitly synthetic reviewer request, two shell
read attempts and final handback appended. This is a fixture, not a live agent
or proof of its understanding. The supplied final assistant text is identical
to the transcript message, avoiding a duplicate supplement.

The schema is the accepted evidence fixture generated from the real AgentJobPacket
Zod source at ai-tools `11d94c986...`; artifact SHA-256 is
`f5e201a09faeb4aea6ef4118cf02903647574cb59f05ce252967103a70c18843`.
No handwritten permissive schema replaces the authority. The evidence fixture
README retains the full generator/source provenance. Fingerprints of reviewed
files, schema and composed transcript are captured before each observation.
They establish identity, not an independent digest-correctness oracle.

Expected IDs, answer names, thresholds and severities come from the reviewed
rules. The harness independently computes weighted score/noul arithmetic from
observed answers, resolves each answer reference into an actual completed
request and checks pass/fail activation. It allows genuine unfavorable answers;
no success verdict is forced. Hook host/event/session/turn/agent type and cursor
are checked as literal provenance. A legitimate uncertain/no-route response
is retained but cannot close the routed V29 journey.

## V14 fixed local measurements

Before measurement, trace 7294 fixed two variants per journey: max_questions=32
versus 2 (skill) or 4 (handoff), with everything else held fixed. Five ordered
first/repeat pairs per variant produce 40 invocations. Preflight proved both
variants admitted and retained the same selected scope before this run began.
Both members start a new process and EvidenceSession. Labels cold_session_first
and warm_path_repeat identify order only: OS cache is uncontrolled, and
fingerprinting/preflight can warm files. No cold storage or model-cache claim.

Fixed settings preserve reviewed request/token/byte limits, max concurrency 4,
speculation off, estimated accounting and no declared provider ceiling. The
harness explicitly overrides reviewed attempt_timeout_ms=10000 to 60000 for
bounded provider headroom, and keeps retry_count=0. This was declared before
measurement and applies to controlled and hosted inputs; it is not a speed
target. Skill max_requests=4; handoff=6.

Run `243ccdfb-5ef1-4c68-a0af-ec1aecea2a25`, exit 0, one measurement test passed,
40 process outcomes completed, 70 observed HTTP requests. Every process retains
the same three target occurrences and full question-owner multiset. Batching
uses 1 rather than 3 requests for skill and 1 rather than 2 for handoff. These
are the declared smaller-budget comparators, not an independently implemented
per-rule engine baseline.

Wall timing starts before spawn and ends after child/stdout/stderr join; it
excludes JSON encoding and artifact writes. Native local/provider time and
read/parse/request/attempt counters are retained separately. Provider time can
sum concurrent work and is not a subtraction component of process wall time.

| Journey | Max questions | Order | Wall min / median / max / sample SD (ms), n=5 | Local median | Provider median | Requests |
| --- | ---: | --- | --- | ---: | ---: | ---: |
| Skill | 32 | First | 112.84 / 117.88 / 786.59 / 299.50 | 0 | 21 | 1 |
| Skill | 32 | Repeat | 116.34 / 118.78 / 125.29 / 4.32 | 0 | 22 | 1 |
| Skill | 2 | First | 109.57 / 115.70 / 130.07 / 8.10 | 2 | 48 | 3 |
| Skill | 2 | Repeat | 118.02 / 122.92 / 126.06 / 3.03 | 2 | 62 | 3 |
| Handoff | 32 | First | 125.97 / 134.05 / 136.63 / 4.55 | 12 | 22 | 1 |
| Handoff | 32 | Repeat | 130.15 / 131.92 / 135.90 / 2.55 | 12 | 22 | 1 |
| Handoff | 4 | First | 134.02 / 135.59 / 174.21 / 17.51 | 15 | 42 | 2 |
| Handoff | 4 | Repeat | 135.59 / 142.36 / 226.64 / 38.60 | 16 | 41 | 2 |

Full rows, all local/provider distributions, receipts and parsed HTTP bodies:
`target/cli-process-observations/21812-v14-controlled.json`. Raw process data:
`target/cli-process-evidence/21812-*`. Log:
`target/cli-phase2-measurements.log`. The first process's 786.59 ms outlier is
retained. Signing/process/OS variability and controlled-provider behavior limit
the comparison. No SLA or favorable speedup is inferred.

## Hosted campaign and bounded diagnosis

Failed campaign `bb3e300f-d3f0-42b4-8306-fb1d078f71f8` remains failed. Its first
handoff process returned incomplete_report/3 after one 60-second timeout, with
remote_outcome_unknown, router_failed and no fabricated receipt. Native
provider time was 61002 ms. The handoff repeat assertion stopped; nextest
fail-fast left skill unrun. Thus only one of four planned invocations ran.
This is not V29/V30 completion, and remote inference/billing remains unknown.
Raw evidence: `target/cli-process-evidence/30224-0`; campaign log:
`target/cli-phase2-hosted.log`.

The Lead requested one bounded diagnosis, no timeout increase. An offline
observing wrapper delegated to the real Jev encoder during prepare_rules and
asserted zero dispatch. It restored identical transcript bytes at the original
path, verified against the failed invocation's frozen digest. The sole encoded
request is 14537 bytes against max_request_bytes=524288; digest
`0d39d25fa9345a4d38b2996be191dcd4318f97c3aeaf9c0a5a228abcbbc981cb`.
It contains seven questions: router choice; finished-handback noul; source-read
noul and count choice; read-only noul; handback score and findings noul. Full
evidence labels and budget counts are in request-characteristics.json beside
the failed process. Encoder source serializes only model/state/questions;
endpoint and credentials do not enter this body.

The diagnostic harness first failed on relative artifact paths and then native
admission from nextest's crate CWD. Correcting the harness to absolute artifact
paths and explicit workspace CWD yielded offline run
`9a4f6b53-d9ba-46a9-9fd7-1fda75776b52`, one passed, zero dispatch.
These were diagnostic setup errors, not additional hosted attempts.

Exactly one direct replay of those encoded bytes, outside the campaign, returned
HTTP 200 in 358.78 ms with an 848-byte response, at the same 60-second deadline.
The response and direct-replay-result.json are retained beside the failed
process; no authorization headers or credential values are recorded. This is
consistent with transient provider latency, not proof of its definitive cause.
Per Lead conditional authorization `private-coordination-record`, a
separately named four-invocation campaign follows, preserving this failed one.

Fresh campaign `3ebef280-6144-4d94-9705-f06c9f357a8d` also failed on its first
handoff invocation: wall 61766.72 ms, native provider 61005 ms, one timeout
attempt with remote_outcome_unknown, incomplete_report/3 and no receipt. The
remaining three invocations did not run. Log:
`target/cli-phase2-hosted-campaign2.log`; per-condition evidence:
`target/cli-process-observations/71980-hosted-handoff-cold_session_first.json`.
No further campaign or timeout increase followed. Two real executable calls
timed out while the one direct Python replay answered quickly; this suggests a
transport/host path difference and leaves definitive cause unknown. Both
campaigns remain failed. At partial target 0ccb736, V29/V30 and genuine hosted
first/repeat measurements remained open. Diagnosis/campaign attempts at that
checkpoint: two timed-out binary attempts plus one successful direct replay,
with no hidden retry.

Lead diagnosis `private-coordination-record`: LuLu is active with
loopback allowed; its rules approve the native test identity but contain no
approval for the CLI identity. Own read-only inspection corroborates that
target/debug/sunstack-rule is Developer ID signed as
`dev.shravansunder.sunstack-rule.debug`, and the LuLu rules contain the
`dev.shravansunder.sunstack-tools-tests.debug` identity but no CLI identity.
Private firewall flow logs were unavailable to the Lead, so host egress is the
leading hypothesis, not an observed drop attributed to LuLu. No firewall rule,
proxy, signing identity or alternate execution path was changed. The successful
Python probe is diagnostic evidence only, never V29/V30 proof.

Partial target 0ccb736 was **blocked: host egress approval for the sunstack-rule
identity**. Lead direction prohibited further binary hosted calls until owner
approval and separate campaign 3 authorization. The later approval and run are
recorded below; neither failed campaign is relabeled as successful.

## Campaign 3 after owner egress approval

Lead authorization `private-coordination-record` reported owner approval
and an all-endpoint LuLu allow rule keyed by the CLI signing identity. Own
read-only preflight found that identity in the updated rules. The final Cargo
build had left a linker ad-hoc signature; the unchanged official runner's
--sign-only path restored the intended Developer ID signature without warning.
Before dispatch, codesign -dv showed `dev.shravansunder.sunstack-rule.debug`
and the expected team, and codesign --verify exited 0. The harness continued
to use that same runner. No source/test/settings change, timeout increase,
alternate identity, proxy or Python substitution was made.

Campaign `cb31f916-9d5a-4931-9961-f8a8fdbfef06` exited 0: two journey tests passed,
four fixed first/repeat invocations completed, no skips, no signing warning.
Both members of each pair started new processes/sessions. All four outcomes
were complete_report/0 with one model request, one completed HTTP 200 attempt
and a genuine admitted receipt for `typesafe/jev-1.14-20261014`. No retry was
configured or observed. This is own V29/V30 executable evidence, awaiting the
Lead's separately commissioned hosted observation and acceptance.

| Journey / order | Wall ms | Native local / provider ms | Input / output tokens | Receipt USD |
| --- | ---: | ---: | ---: | ---: |
| Handoff first | 1033.78 | 14 / 355 | 5432 / 209 | 0.000228144 |
| Handoff repeat | 445.44 | 13 / 338 | 5432 / 209 | 0.000228144 |
| Skill first | 448.92 | 0 / 352 | 2342 / 139 | 0.000098364 |
| Skill repeat | 521.65 | 0 / 430 | 2342 / 139 | 0.000098364 |

Campaign 3 receipts total USD 0.000653016. That is only the four admitted
receipts, not a total including the earlier unknown remote outcomes or the
separate direct probe. These are single ordered pairs per journey, not a
population estimate. OS/provider cache state and signing/startup overhead are
uncontrolled; the first handoff wall value is retained. No speedup, SLA,
cold-storage state or provider-cache state is inferred.

Handoff routed to review_handback at confidence 0.85 / 0.80. Both reports
retained the reviewed hook host/event/session/turn/subagent provenance and one
cursor. The three judgments have block/block/nudge severities and transcript
record locations: source reads [tool:review-read-0, tool:review-read-1], changes
[tool:first, tool:review-read-0, tool:review-read-1], handback [message:2571].
Their observed values were 0.98 / 0.98 / 0.9165 first, and
0.98 / 0.98 / 0.9145 repeat, all pass. Informational answers resolved to actual
completed request answers. Their correctness as semantic judgments is not
measured beyond the executable/contract journey and arithmetic checks.

Skill reports retained U001/U002/U003, byte spans [22,85), [87,190), [192,272)
in the reviewed SKILL.md and nudge/nudge/block severities. Observed values were
0.93 / 0.985 / 0.97 first, and 0.92 / 0.9825 / 0.97 repeat, all pass.
task_kind, specificity and names_file references resolved into the genuine
answers. The independent arithmetic and source-byte checks passed; favorable
verdicts were not a precondition of the harness.

Log: `target/cli-phase2-hosted-campaign3.log`. Full reports, native counters,
receipts and pre/post fixture fingerprints:
`target/cli-process-observations/31999-hosted-handoff-*.json` and
`target/cli-process-observations/32116-hosted-skill-*.json`. Raw process evidence:
`target/cli-process-evidence/31999-{0,1}` and `32116-{0,1}`. Frozen fixture
fingerprints stayed unchanged. The immediate change from the two 61-second
timeouts to four successful executable calls after owner approval corroborates
the egress hypothesis; private firewall flow logs still were not observed.

The observed source/test tree is the frozen local-verification target 0ccb736.
This proof update is documentation only; that commit remains immutable for the
paired Verifier's ongoing independent local observation. The full phase 2
declaration adds this account without claiming independent verification or
Lead acceptance. V2/V13/V14/V29/V30 have own scoped evidence, with the
PreparationFailed reachability, cache, semantic accuracy and integration limits
kept explicit above.

## Gates and remaining boundaries

All Cargo commands use the command-local SDKROOT/CC/CFLAGS prefix from the root,
as in the phase 1 account. Intermediate CLI gate
`3a141398-90fb-43cb-979f-081f89e26862`: 23 passed, three deliberate
hosted/measurement skips, exit 0; `target/cli-phase2-current-local.log`.

Final local gates on source checkpoint `7af7cf1` (only this proof account is
updated afterward):

| Command | Result | Evidence |
| --- | --- | --- |
| cargo fmt --all -- --check | Exit 0 | target/cli-phase2-final-fmt.log |
| cargo clippy --workspace --all-targets -- -D warnings | Exit 0, zero warnings | target/cli-phase2-final-clippy.log |
| cargo build -p sunstack-rule-cli | Exit 0 | target/cli-phase2-final-build.log |
| cargo nextest run --workspace | Exit 0, 168 passed, 7 skipped, no LEAK marker | target/cli-phase2-final-workspace.log |

Workspace run ID `cd668eb4-419b-48ad-99de-08b05e6d5e49`. The seven default skips
are three native hosted cases, two CLI hosted journey cases, the explicit
40-process measurement case and the offline preserved-request diagnosis.
The latter two were separately executed as described above. The real hosted
journey cases were attempted in two failed campaigns, not silently waived.

The independently observed phase 1 native decisions LEAK remains a separate
Lead-owned investigation. A later run without a LEAK marker does not prove it
fixed. The executable/milestone-1 verification recipes remain DRAFT; activation
belongs to the separately scoped maintenance owner. No cross-platform, live
host hook, production skill integration, deployment or release is claimed.
