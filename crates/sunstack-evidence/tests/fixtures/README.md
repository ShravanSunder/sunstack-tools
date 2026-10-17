# Fixture provenance and independent oracles

Claude fixtures are exact `data` dictionaries from `tests/test_message_parser.py` in
anthropics/claude-agent-sdk-python at `c10b52c674e5c709c7263544b38e8a5dad10e01d`.
The four named `test_parse_user_message_*` functions were parsed with Python `ast`,
the `data` assignments evaluated with `ast.literal_eval`, and serialized as JSONL.
Their content is unchanged; JSON formatting differs. MIT license: `LICENSE.claude-sdk`.

Source: https://github.com/anthropics/claude-agent-sdk-python/blob/c10b52c674e5c709c7263544b38e8a5dad10e01d/tests/test_message_parser.py

| Fixture | Oracle from literal content and Specification E31/C11 |
| --- | --- |
| `test_parse_user_message_with_tool_use.jsonl` | 2 records: one user message and `tool_456` / Read / read_file / result_missing / absent |
| `test_parse_user_message_with_mixed_content.jsonl` | 2 records: one user message retaining both text blocks in order; `use_1` / Search / search / succeeded / Search results |
| `test_parse_user_message_with_tool_result.jsonl` | 0 records, orphan_result for tool_789 at byte 0 |
| `test_parse_user_message_with_tool_result_error.jsonl` | Alone it is an orphan. In the explicitly synthetic preceding-call composition it completes tool_error with failed / File not found |

The mixed count follows Lead disposition, activity 6901 on execution root
`private-coordination-record`. The earlier experiment counted text blocks
as separate events. That historical count is superseded by whole-message identity.

`codex-public.jsonl` copies the first three raw JSON literals verbatim from
`rollout_line_decoder_preserves_canonical_json_compatibility` in openai/codex,
`codex-rs/rollout/src/tests.rs`, commit `3bae578af33e43268e7bc2d6d5d0266f704bb554`.
Oracle: 0 projected records and 3 unsupported_record gaps, at the exact byte start
of each literal. These are token-count, developer-message and warning records.

Source: https://github.com/openai/codex/blob/3bae578af33e43268e7bc2d6d5d0266f704bb554/codex-rs/rollout/src/tests.rs#L54

`codex-composed.jsonl` is **synthetic composition**: the `first` function-call
payload and `first`/`Sent.` output payload from
`recorded_questions_share_queued_input_order_across_resume` in
`codex-rs/core/src/session/rollout_reconstruction_tests.rs` at that same Codex pin
are wrapped in explicitly synthetic timestamped rollout envelopes.
Oracle: 1 paired tool record, native call ID first, mcp category,
result_present_status_unknown, full observed field Sent. The result string does
not state success. Apache-2.0 license: `LICENSE.codex`.

Source: https://github.com/openai/codex/blob/3bae578af33e43268e7bc2d6d5d0266f704bb554/codex-rs/core/src/session/rollout_reconstruction_tests.rs#L34

All other conversations assembled inside the tests are **synthetic**. They probe
streamed blocks, late results, incomplete tails, consecutive users, branch links,
injection attribution, supplements, windows and invalid adapter outputs. They
are not live-host qualification. Known clipped sizes are tested at the adapter
contract boundary; no public fixture establishes a real host's original byte count.

## AgentJobPacket schema

The JSON Schema fixture was generated with Bun and Zod 4.6.5 from the actual
AgentJobPacket union and its transitive imported role contracts. It was not
handwritten or edited after generation. The original source worktree was removed
by another session after generation. Its source remains retrievable in the
ai-tools Git repository at `11d94c9869b0b8e29d8e18687473c638942c8245`.

Git-relative source:
`plugins/sunstack-dev/skills/practices-manage-agents/schemas/agent-job-packet.ts`.
SHA-256: `7c5673c715ce824515421f99410b539398ae65ccde819385b5a484465cbaa5bb`.
That hash was independently rechecked with `git show <commit>:<path> | shasum -a 256`
after the worktree disappeared. The source was read-only throughout.

Exact original generation command (at the evidence worktree root):

```sh
bun -e 'import * as z from "zod@4.6.5"; import { AgentJobPacket } from "private verification archive; console.log(JSON.stringify(z.toJSONSchema(AgentJobPacket, { target: "draft-2020-12" }), null, 2));' > crates/sunstack-evidence/tests/fixtures/agent-job-packet.schema.json
```

To reconstruct its source elsewhere, export the entire `schemas/` subtree from
that Git commit into a disposable directory, preserving the relative imports,
and substitute the exported entrypoint in this command. No write to ai-tools is
needed. The generated schema has 7 role variants. The positive reviewer packet
and negative role/camelCase/author-history controls are in `payload_inputs.rs`.
The test exercises the real offline jsonschema 0.57 validator; registered local
references pass and unregistered HTTP/file references fail compilation.

Generated schema SHA-256: `f5e201a09faeb4aea6ef4118cf02903647574cb59f05ce252967103a70c18843`.
