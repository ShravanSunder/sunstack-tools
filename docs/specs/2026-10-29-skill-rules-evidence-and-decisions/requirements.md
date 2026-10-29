# Skill rules on evidence and decisions: requirements

Skill rules check a skill against the skill-writing rulebook and point at the exact lines that fall short. Today they can't do that well. Each rule is a tagged rulebook sentence turned into one yes/no question, its target is a raw Markdown unit (a section, paragraph or list item), and the engine forces every judged rule into a plain yes/no verdict with no structure. A rule can't see a whole step with its completion line, can't ask a choice or score question, can't combine answers, and can't say "uncertain". The rules runner in sunstack PR #171 needs a stand-in binding file just to pass admission, and its frontmatter checks can't run as judged rules at all.

This change cuts each skill along its Markdown structure with ast-grep, groups the related parts into evidence (a whole step, a load line with the file it loads, the description), and runs each rule's decision on that evidence: mechanical checks with no AI first, then questions to the decision model in its own structured form, with the rule's thresholds turning the answers into pass, finding, not applicable or uncertain. A config file holds the inputs and the evidence system; rule files hold the decisions. There are no rule sets for this path.

It replaces the Markdown-marker route of the [Markdown skill rules design](../2026-10-17-markdown-skill-rules/requirements.md) (its U1–U10) and extends the [Rust rule engine](../2026-10-12-rust-rule-engine/requirements.md). It keeps the [sunstack runner](../2026-10-18-sunstack-runner/requirements.md)'s one binary with `lint` as its command (U1–U3), its `error`/`warn` vocabulary (U4) and Jev as the decision model (U7). It supersedes that design's U5–U6 on one point only: lint's config points at rulebook files and their rule files, not at rule sets. The [Specification](specification.md) defines the contract, the [Program Design](program-design.md) places it in the crates, and the [Verification Design](verification-design.md) plans its proof. The owner confirmed the design on 29 Oct 2026 from the page *Skill rules: how the system should work*.

## Who it serves

| Who | Job | What's missing today |
| --- | --- | --- |
| The agent running the check: skill creation's self-check, skill review, skill audit | Check a skill (or a PR's changes) against the rulebook and act on the result | Findings point at raw units, not steps; there's no "uncertain" to hand back; a judged rule can't look at a step and its completion line together |
| Rulebook author (the owner, or an Opus Worker writing skills) | Write a standard once, tagged in the rulebook, and say how it is decided | A rule is one yes/no threshold; no choice or score questions, no combined answers, no mechanical first pass in the same rule |
| The rules runner in sunstack `tests/skills` | Run every rule on every skill, or on a PR's changes, with the decision model on | It needs RuleSets, a stand-in binding file, and can't pick by what a PR changed |
| CI | Refuse a broken rule or config before it merges | The check needs fake bindings and can't validate decision structure |
| The owner | Trust that the decision model only ever sees a bounded, relevant piece of a skill | Requests are formed from whole units and binding groups, with no size cut |

## The owner's words (28–29 Oct 2026)

- "jev is teh whole fuckign poitn", "we wnat to use decion models eveyrwehre", "fix the fuckign engine" (28 Oct)
- "every skilw as supose dot ahve skill ruels there were some thigns borken ins unstack-toosl youw ere skuppse dto fix in teh i scli"
- "also the engien for jev must accetp strucutre not jus tnoul", "the nul is configraible and the ratio", "https://docs.typesafe.ai/primitives/advanced", "this is teh stuff to fix nto buildign a eval hanress in rust liek slop"
- "inputs -> filers system -> coales all iptus -> filter -> evience / then diecions system"
- "are they chuknekd jev cant review a entire file", "cannot" (feed a giant file)
- "Unsureshown for a person to read woudl you not jnot pass this as model as uncertain?"
- "we have config and rule files right? skill.rules.config.json? and then rulesets or whtever?", "so config has inputs and evicdence system? and rules has how to work with that evidence?"
- "cut into pecies shoudl be ast grep no? and deicion and deicion models should be the name"
- "jev can parse astgrep and html we shoud lbase kour system aorukdn astgrep features", "in this first step onlymarkdown for skill rules"
- on where the common ast-grep pieces live: "you mean in sustack tools repo thats very videly common"
- the go: "sokudns godo then we can build this then make pr, then use it for 170 (use lcoal build to test it make sure it worsk prove it with allc reation skills ...)"
- "session pipel lien is alater id ont want you to build ti till later"

## Authorized needs

| ID | Need | Authority |
| --- | --- | --- |
| U1 | A skill is cut along its Markdown structure by ast-grep, with no AI and no Markdown parser of our own. Each kind of evidence is defined as an ast-grep rule, written in ast-grep's own rule language. | "cut into pecies shoudl be ast grep", "base our system around astgrep features"; confirmed page, adjustment 1 |
| U2 | Related parts are grouped into evidence before anything is judged: a whole step with its heading, body and completion line; a load line with the file or skill it names; the description with the step headings; a branch; a sentence with the headings above it; a reference section. The decision model never gets a whole file, and every piece of evidence has a size limit. | "inputs -> filters -> coalesce all inputs -> filter -> evidence", "jev can't review an entire file"; confirmed page, adjustments 2, 4–7 |
| U3 | The widely common Markdown pieces (a numbered heading section, a bold numbered item, a block quote, the frontmatter) are written once as a standard ast-grep util library that ships with the sunstack CLI. A repository's own conventions (what a step, completion, load and branch are) live in that repository, built from those pieces, and its config points to them once. | "you mean in sunstack tools repo, that's very widely common"; confirmed page, "Where the ast-grep rules live" |
| U4 | One config file holds the inputs (which rulebook files pair with which rule files, which files to check), the evidence system, and the run settings (the decision model and its key, size limits). Rule files hold, for each tagged rulebook sentence, the evidence kind it judges, its questions and its decision. There are no rule sets on this path. | "config has inputs and evidence system, and rules has how to work with that evidence"; confirmed page, "The files" |
| U5 | A rule's decision is a set of named steps: mechanical checks with named tools and no AI (ast-grep, regex, contains, max characters, file exists), questions to the decision model, combining answers, and the rule's own thresholds. Its result is pass, a finding at a line, not applicable, or uncertain. | "decision and decision models should be the name", "the null is configurable and the ratio"; confirmed page, "5 and 6" |
| U6 | Questions use the decision model's own structured schema, unchanged: yes/no (noul), choice and score questions with structured instructions and criteria. A rule's questions about one piece of evidence go in one request. | "the engine for jev must accept structure not just noul", "docs.typesafe.ai/primitives/advanced" |
| U7 | Every rule with questions runs through the decision model. There is no switch that turns it off and no path that skips it. | "jev is the whole point", "use decision models everywhere" (28 Oct) |
| U8 | An uncertain result goes back to the agent running the check, with the evidence and the answers, and that agent decides. Nothing waits for a person. | "would you not pass this as model as uncertain?"; confirmed page, "Uncertain" |
| U9 | A check validates the config, the tags and every rule file (evidence kinds, questions, decisions, ast-grep rules) before anything runs, reading no skill and asking the model nothing, with no stand-in inputs. | "things broken in sunstack-tools you were supposed to fix in the CLI"; confirmed page, "How a rule entry is built" |
| U10 | A run picks evidence by kind, and on a PR only the evidence whose lines the PR changed (a load also when the file it loads changed). An audit picks everything. | "on a PR only the evidence the PR touched"; confirmed page, "How picking works" |
| U11 | The engine stays generic: which sentences are tagged, the evidence conventions, the questions and thresholds are the consumer's content. | Engine U9 (consumer-authored rules) |
| U12 | At runtime an invalid rule is reported with its diagnostics and skipped, and the remaining rules still run; the gap is visible in the result. | 2026-10-17 U5, owner: "on runtime we log validation errors, but work and ignore rules that fail" |

## What carries over from earlier designs

| Earlier row | Here |
| --- | --- |
| 2026-10-17 U2 (everything but wording in a rule file keyed by marker), U3 (runtime assembly, no generated file), U4 (check flag, tags both ways), U6 (allowed hosts, size cap), U7 (hidden content stripped) | carried unchanged in meaning (U4, U9 and the Specification's R5) |
| 2026-10-17 U5 (invalid rules skipped at runtime) | carried as U12 |
| 2026-10-17 U10 (engine stays generic) | carried as U11 |
| 2026-10-17 U1 (the bound text is the question) | superseded on 29 Oct: the tagged text is the *standard* every request carries; the questions are authored in the rule file in the decision model's schema |
| 2026-10-17 U8 (each section, paragraph, list item or call site judged as its own occurrence) | superseded on 29 Oct by evidence pieces (U2) |
| 2026-10-17 U9 (mechanical forms `expect` and `ast_grep`; file existence stays a CI script) | superseded on 29 Oct by mechanical check steps with named tools, `file_exists` included (U5) |
| 2026-10-18 U4 (`error`/`warn`), U7 (Jev now, a local model later in the same config) | carried |
| 2026-10-18 U5–U6 (lint config points at rule sets) | superseded on 29 Oct: lint's config points at rulebook files and rule files, with no rule sets |

## Non-goals

- The session pipeline (session logs as inputs, scenario generation). Later, after much more discussion.
- An eval harness, scoring system or calibration campaign in Rust. Rule tests live in sunstack `tests/skills` (vitest-evals) and run through this CLI.
- HTML evidence (plan and design islands). Markdown only in this step; HTML comes later on the same mechanism.
- The runner, the rule entries, the evidence conventions, the labelled cases and the rule tests in sunstack. They are sunstack PR #171's, which consumes this.
- Generic JSONL RuleSets (`rule_paths`) and conversation rules. They keep their meaning; this change replaces only the Markdown-marker route.
- Releasing or tagging the CLI. The owner releases.

## Goal boundary

- **Goal:** the sunstack CLI runs skill rules on ast-grep evidence with structured decisions, so #171 can check every skill-authoring creation skill on a local build with the decision model on.
- **Reuse:** the engine's ast-grep Markdown parse (tree-sitter-md), the decision model contract and its provider call, credential resolution, the `<!-- K001 -->` marker form, and the engine's change scopes.
- **May change:** the sunstack-tools crates `sunstack`, `sunstack-lint`, `sunstack-rules`, `sunstack-evidence`, `sunstack-decisions`, plus the standard util library as data files shipped with the CLI.
- **Protected:** the session-pipeline branches, the `exec` protocol's JSONL rules and conversation rules, credential storage, Demistify.
- **Complexity:** no new Markdown parser, no rule language of our own beyond the decision steps, a small evidence builder (ranges, captures, one YAML read, size limits).
- **Evidence of done:** the two worked examples on the confirmed page produce their stated results through the CLI, and every skill-authoring creation skill is checked on a local build with the decision model on.
