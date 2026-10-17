# Markdown skill rules: requirements

Today a skill rule is hand-written JSONL. Its question repeats what the skill teaching that standard already says, so the two copies drift. A Markdown target must carry an authored `U001:` paragraph marker, one rule gives one verdict for a whole binding group, and every verdict needs a model call.

This change lets a skill mark the text that teaches a standard. The engine then assembles the rule from that text and a small rules file at runtime, judges each section or call site of the skills under review, and settles mechanical checks without a model.

It extends the [Rust rule engine](../2026-10-12-rust-rule-engine/requirements.md) and keeps its U1–U28. The [Specification](specification.md) defines the contract, the [Program Design](program-design.md) places it in the crates, and the [Verification Design](verification-design.md) plans its proof.

## Who it serves

| Who | Job | What's missing today |
| --- | --- | --- |
| Skill author (a person, or an Opus Worker writing skills) | Write a standard once, in the skill that teaches it, and have it checked everywhere | The standard must be copied by hand into JSONL, and nothing notices drift |
| `skill-audit` / `skill-rules` (Sunstack) | Run the skill rules over every skill and read source-linked findings | Targets need `U001:` markers, and a rule gives one verdict per group |
| CI | Refuse a broken rule before it merges | No check that runs every rule's admission without running the rules |
| The owner | Trust that a rule's question is the skill's own visible words | No guarantee that what the model is asked matches what a reader sees |

## The owner's words (2026-10-17)

- "rust tool has astgrep, so we can use and build this into the tool itself"
- "i would want things to be more deterministic"
- "if we have 2 sources of truth the actual skill and rule file, we might as well handcraft the jsonl"
- "so a token u001> then the paragraph is the instruction -> the rest is in the correlated instruction file"
- "i think i like no generated check in file as long as we have a compile check flag. and on runtime we log validation errors, but work and ignore rules that fail. we can put stuff in ci as well to make sure they pass."
- "the rule check should look for tags that exist and not in rules, or tags in rules not in markdown"
- on the marker form, picking `<!-- K014 -->` from the measured options: "this is good"; and "we can separate the K011 from the name in the rule and add that as name: skill.call.complete but don't need to burn tokens there"
- "so hierarchy matters ... we refuse to do it for certain bounds? like h1?" and "or certain types of markdown types for safety?", then "sounds good" to the allowlists and size cap

## Authorized needs

| ID | Need | Authority |
| --- | --- | --- |
| U1 | A rule's wording lives once, in the skill text that teaches the standard. A marker at the end of that text's line binds it, and the bound text is the question the model is asked. | Owner: "2 sources of truth", "the paragraph is the instruction" |
| U2 | Everything that isn't wording (name, target, threshold, severity, check, extra context) lives in a rules file keyed by the marker ID. The marker carries only the ID. | Owner: "the rest is in the correlated instruction file", "separate the K011 from the name" |
| U3 | The engine assembles rules at runtime from the skill Markdown and its rules file, deterministically, using the Markdown syntax tree. No generated rule file is checked in. | Owner: "build this into the tool itself", "more deterministic", "no generated check in file" |
| U4 | A check flag runs full assembly and admission without judging anything, for authors and CI. It fails on any invalid rule, any marker without a rules entry, and any rules entry without a marker. A print flag shows each assembled rule exactly as the model would get it. | Owner: "compile check flag", "put stuff in ci", "tags that exist and not in rules, or tags in rules not in markdown" |
| U5 | At runtime, an invalid rule is logged, skipped and listed in the report. The remaining rules run. | Owner: "on runtime we log validation errors, but work and ignore rules that fail" |
| U6 | Markers bind only allowed hosts: `##`–`######` headings (the whole section), paragraphs, and list items. H1 and every other node type are refused. A bound text has a size cap. | Owner: "refuse ... like h1", "certain types of markdown types for safety", "sounds good" |
| U7 | The question contains only what a reader sees in the rendered skill. Hidden content (HTML, comments, link definitions) is stripped. | Owner: "for safety", "sounds good" |
| U8 | A rule judges each matching section, paragraph, list item or call site of a reviewed skill as its own occurrence, reported at file and line. | Engine customer: "the location of the paragraph to repair"; spike on five real skills |
| U9 | A mechanical check (a referenced file exists, a pattern is present or absent, a frontmatter field is present) settles without a model call or credential. | Provisional: the Lead's proposal for "more deterministic". The owner hasn't confirmed it on its own. |
| U10 | The engine stays generic. Which standards get markers, their thresholds and severities, and the marker letter are the consumer's content. | Engine U9 (consumer-authored RuleSets) |

## Non-goals

- Which skills get markers, the decision matrix for what becomes a rule, the `skill-rules` / `skill-audit` work, and evals. All of that is Sunstack's.
- Hosted or paid model runs and calibration campaigns.
- Changing what existing `rule_paths` JSONL rules, `U001:` paragraph targets or current verdict kinds mean. Everything here is additive.
- A local model backend, and new host adapters.

## Open owner decisions

- **U9 as a requirement.**
- **The job of a standard's good and bad examples.** The engine contract doesn't depend on it.
