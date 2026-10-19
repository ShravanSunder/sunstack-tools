# sunstack runner: Specification

What must be observably true of the `sunstack` binary, its `sunstack lint` command, the `sunstack.lint.json` config, its builds, and its install channels. [Requirements](requirements.md) authorize every obligation here. The rule engine's own [Specification](../2026-10-12-rust-rule-engine/specification.md) still governs what a rule means, how evidence is selected, and how judgments are made; this document only adds the runner around it.

## Capability map

Which obligations belong to each confirmed capability? Names and purposes re-render the [confirmed list](requirements.md#capabilities).

| Key | Capability | Purpose | Home obligations | Also serves |
| --- | --- | --- | --- | --- |
| `runner` | The runner | One `sunstack` command for lint now and other uses later, from a terminal, Node and Rust | [R1–R7](#runner) | — |
| `config` | The config | One file per use: what to run, at what severity, on which files, with which backend | [R8–R15](#config) | R14 also serves `runner` |
| `portable-builds` | Runs anywhere | Binaries that run on Linux and macOS with nothing else installed | [R16–R18](#portable-builds) | — |
| `install` | Install it the usual way | Homebrew or whatever people normally use | [R19–R23](#install) | — |

The path from need to proof is [Requirements](requirements.md) → this Specification → Program Design (not yet authored) → Verification Design (not yet authored).

## Who uses which surface

```mermaid
flowchart LR
  person["You or an agent at a terminal"] -->|"sunstack lint"| S(["sunstack"])
  ci["CI job"] -->|"sunstack lint --format github or sarif"| S
  node["Node / TS caller"] -->|"npx sunstack lint --format json"| S
  rust["Rust caller"] -->|"library call, same report"| S
  prog["Program using the protocol"] -->|"sunstack exec (JSON in, JSON out)"| S
  S -->|"reads"| cfg[("sunstack.lint.json and rule-set files")]
  S -->|"judged rules only: selected text"| or[("OpenRouter, Jev")]
  S -.->|"never"| none["writes to the repository, telemetry, other network"]
```

## Problem

Today the engine runs only from a hand-written JSON request on standard input (`crates/sunstack-rule-cli/README.md`). There is no config file, no command that lints a repository, and no exit code tied to severity: a failed rule still exits 0. The macOS build links Homebrew's libgit2, so it fails on a Mac without it, and there is no release channel, so using the engine means building from source (`/private/tmp/2026-10-runner-distribution/experiment/report.md`, H1). Rule files say `block` and `nudge`, while lint users expect `error` and `warn`.

| ID | Problem | Outcome |
| --- | --- | --- |
| P1 | A repository cannot check itself with one command; every run needs a hand-built request, and the exit code ignores failures | O1: one command and one checked-in config lint a repository and return an exit code that reflects what failed |
| P2 | There is no installable build, and the existing build is tied to the machine that built it | O2: anyone on Linux or macOS installs a working binary through the usual channels |
| P3 | Severity words differ between rule files and lint convention | O3: one vocabulary, `error` and `warn`, everywhere |

## Entities

| ID | Term | Identity rule | Relationships | Invariants | Observable states | Basis |
| --- | --- | --- | --- | --- | --- | --- |
| E1 | Lint config | The file named by `--config`; otherwise the nearest file named `sunstack.lint.json` found by walking up from the current directory. Two runs share a config when they resolve the same file path. | Has 1..n rule-set entries (E2), 0..n overrides (E5), 0..1 backend selection (E8) | Holds no rule definitions. Relative paths in it resolve from its own directory. Unknown fields are refused. | `valid`, `invalid` | U5, U6 |
| E2 | Rule-set entry | Its name, unique within one config | Belongs to one E1; references one rule-set file; has 1..n file globs for that rule set's input | The referenced rule set declares exactly one input, of kind `document`. The entry name prefixes the IDs of its rules. | `valid`, `invalid` | U5, U6 |
| E3 | Rule | The rule-set entry name plus the rule's own ID, written `<entry>/<rule id>`, such as `skills/skill.description`. The same rule in two configs is the same rule only when both point at the same rule-set file. | Belongs to one rule set; has one default severity; has one effective severity per file | Its definition lives only in a rule file. | `enabled`, `off` for a given file | U5, U6 |
| E4 | Severity | One of `error` or `warn` in rule files; config adds `off` | Each rule has one default; each finding has one effective severity | Never sent to the model. A rule with no severity is `warn`. | — | U4; engine U26 |
| E5 | Override | Its position in the config's `overrides` list | Belongs to one E1; has 1..n file globs and 1..n rule settings | Applied in list order; a later match wins | — | U5 |
| E6 | Lint run | One invocation of `sunstack lint`: one config, one set of target files, one change scope | Uses one E1; produces 0..n findings (E7) and one report | Reads the repository; never writes to it | `clean`, `findings`, `invalid`, `incomplete`, `cancelled` | U1 |
| E7 | Finding | One rule (E3) failing at one location in one file | Belongs to one E6; has one effective severity | A rule turned `off` for a file produces no finding there | `error`, `warn` | U1, U4 |
| E8 | Backend selection | Its kind plus model and endpoint | Belongs to 0..1 E1 | Holds no credential; the key comes only from the process environment | — | U7 |
| E9 | Release | Its version number | Has four platform builds (E10); is published to every install channel (E11) | Every channel serves the same bytes for a target | `draft`, `published` | U10, U11 |
| E10 | Platform build | Release version plus target: macOS arm64, macOS x64, Linux x64, Linux arm64 | Belongs to one E9 | Runs with only the operating system's own libraries | — | U8, U9 |
| E11 | Install channel | One of: Homebrew tap, shell installer, GitHub Release archive, npm package | Serves every E9 | Installs the matching E10 unchanged | — | U10 |

## runner

| ID | Requirement | From |
| --- | --- | --- |
| R1 | `sunstack lint [paths…]` loads the governing config (E1), runs every enabled rule over the files its rule-set entries select, limited to `paths` when given, and reports findings (E7) with rule, file, location, severity and message. | U1 |
| R2 | The exit code of a lint run reflects its outcome, as set out in [C2](#c2-exit-codes). | U1 |
| R3 | `--format` selects `default` (for people), `json` (stable, for agents and Node), `github` (workflow annotations) or `sarif` (SARIF 2.1.0, for code review). Every format carries the same findings. | U1, U2 |
| R4 | `--changed` limits a run to files with uncommitted changes, and `--since <rev>` to files changed since a revision. Without either, the run covers every selected file. | U1 |
| R5 | `sunstack lint --check` validates the config and every rule set it names without running a rule or making a model request. `--rules` lists each rule with its entry, kind (mechanical or judged) and effective default severity. `--init` writes a starter `sunstack.lint.json` and refuses to overwrite an existing one. | U1, U5 |
| R6 | `sunstack exec` keeps today's JSON protocol: one request on standard input, one response on standard output, exit codes 0, 2, 3 and 4 with today's meanings. Only the severity words change (R14). | U3 |
| R7 | The same lint is callable from Node through the npm package (`npx sunstack lint --format json`) and from Rust as a library call that returns the same report as `--format json`. | U2 |

### C1. Commands

```text
sunstack lint [paths…] [--config <file>] [--changed | --since <rev>]
              [--format default|json|github|sarif] [--max-warnings <n>]
sunstack lint --check  [--config <file>]
sunstack lint --rules  [--config <file>]
sunstack lint --init
sunstack exec
sunstack --version | --help
```

`sunstack` with no command prints help and exits 2. Later uses add sibling commands; `lint`'s flags never change meaning when they do.

### C2. Exit codes

| Code | Meaning | Precedence |
| --- | --- | --- |
| 2 | Invalid input: bad flags, no config found, invalid config or rule set, a judged rule enabled with no usable backend or key | highest |
| 4 | Cancelled by the caller (Ctrl-C or SIGTERM) | |
| 1 | At least one `error` finding, or more `warn` findings than `--max-warnings` | |
| 3 | Incomplete: some enabled rule could not be evaluated on a selected file (selection failed, evaluation failed, not started), a rule set was skipped, or rule coverage was not full | |
| 0 | Clean: none of the above | lowest |

A run with both an `error` finding and an incomplete part exits 1, and its report still lists what was incomplete. `not_applicable`, `outside_change_scope` and a rule that matched nothing are not incomplete.

### C3. Report

The `json` report contains: the config path; the version; each finding (rule, file, line and column span, effective severity, message, and for judged rules the model's answer and the evidence it saw); the incomplete parts with their reasons; counts by severity; and, when judged rules ran, model requests made, tokens and cost as the engine reports them. Field names are fixed within a major version; new fields may be added.

## config

| ID | Requirement | From |
| --- | --- | --- |
| R8 | Without `--config`, lint uses the nearest `sunstack.lint.json` walking up from the current directory. If none is found, it exits 2 and names the file it looked for. | U6 |
| R9 | The config follows [C4](#c4-sunstacklintjson). An unknown field, a missing rule-set file, a duplicate entry name or a bad glob exits 2 with the file, line and column. | U5, U6 |
| R10 | A rule's effective severity for a file is its default from the rule file, then the config's `rules`, then each matching override in order, the last one winning. Within one `rules` map, an exact rule ID wins over `<entry>/*`. | U4, U5 |
| R11 | A rule-set entry may only name a rule set with exactly one `document` input. `--check` refuses any other rule set and names its input kinds. | U5 |
| R12 | Lint skips files that match `ignorePatterns` or that Git ignores. | U1 |
| R13 | A config whose enabled rules are all mechanical runs with no backend and no key. If any enabled rule is judged, lint needs `backend` in the config and `OPENROUTER_API_KEY` in the environment, and exits 2 before any model request when either is missing. The key is never written to output. | U7 |
| R14 | Rule files use `error` or `warn` for severity, and a rule without one is `warn`. `block` and `nudge` are refused with a message naming the replacement. Reports, schemas and documentation use the same words. | U4 |
| R15 | Each release publishes the JSON schema of `sunstack.lint.json` at a versioned URL, and the binary validates configs against the same definition. | U5 |

### C4. sunstack.lint.json

```json
{
  "$schema": "https://github.com/ShravanSunder/sunstack-tools/releases/download/v0.1.0/sunstack.lint.schema.json",
  "ruleSets": {
    "skills": { "path": "rules/skill-standards.ruleset.json", "files": ["plugins/**/SKILL.md"] }
  },
  "rules": {
    "skills/*": "error",
    "skills/skill.load-result": "warn"
  },
  "overrides": [
    { "files": ["plugins/**/retired-skills/**"], "rules": { "skills/*": "off" } }
  ],
  "ignorePatterns": ["tmp/**"],
  "backend": { "kind": "jev_openrouter", "model": "jev-latest" }
}
```

| Field | Required | Meaning |
| --- | --- | --- |
| `$schema` | no | Schema URL for editor completion; not used for validation |
| `ruleSets` | yes | Entry name → rule-set file path and the globs whose files bind to its one document input |
| `rules` | no | `<entry>/<rule id>` or `<entry>/*` → `error`, `warn` or `off` |
| `overrides` | no | Ordered list of file globs plus a `rules` map, applied to matching files |
| `ignorePatterns` | no | Globs lint never reads |
| `backend` | when a judged rule is enabled | `kind` (`jev_openrouter` now), `model`, optional `endpoint`; later kinds, such as a local model, are added beside it without changing rule files |
| `limits` | no | Caps on files read and model requests per run; the engine's defaults apply otherwise |

A rule turned `off` everywhere is not run at all and costs no model request. A rule turned `off` for some files produces no finding on them.

## portable-builds

| ID | Requirement | From |
| --- | --- | --- |
| R16 | Each release has builds for macOS arm64, macOS x64, Linux x64 and Linux arm64. Linux builds load no shared library at all; macOS builds load only libraries the operating system ships. Each runs `--version`, `lint --check` and a lint run on a clean macOS, Alpine and Debian system. | U8 |
| R17 | Judged rules reach OpenRouter on a system with no CA certificate bundle. Certificates from a private CA in the operating system's trust store are not used; this limit is documented. | U8 |
| R18 | Git access uses git2-rs with libgit2 compiled into the binary and no Git network features. | U9 |

## install

| ID | Requirement | From |
| --- | --- | --- |
| R19 | Each release publishes, for every target, an archive and its checksum on GitHub Releases. | U10, U11 |
| R20 | `brew install shravansunder/tap/sunstack` installs the release build on macOS (arm64, x64) and Linux (x64, arm64). | U10 |
| R21 | A one-line shell installer installs the release build on macOS and Linux, and can pin a version. | U10, U11 |
| R22 | `npm i -D sunstack` followed by `npx sunstack …` runs the release build. The package fetches the build at install time; the README states that this needs network access to GitHub and that pnpm may need build approval. | U2, U10 |
| R23 | `sunstack --version` reports the release version, which equals the Git tag and the version on every channel. | U10, U11 |

## Cross-cutting obligations

| Quality | Obligation |
| --- | --- |
| Security | The OpenRouter key comes only from the environment, is never written to output, a report or a config, and is sent only to the configured backend. |
| Privacy | Text selected for a judged rule is sent to OpenRouter. Mechanical rules send nothing. The README says this. |
| Cost | Judged rules cost money per request. `--changed`, `--since`, `limits` and `off` are the controls, and the report shows requests, tokens and cost. |
| Reliability | A model or network failure makes the run incomplete (exit 3), never clean. |
| Compatibility | `exec` keeps its protocol apart from the severity words. `sunstack.lint.json` is versioned by its schema URL; a breaking change needs a new major version. |
| Performance | Not specified: the owner set no threshold. |
| Accessibility, data lifecycle, compliance | Not applicable: a CLI that reads files and writes reports. |

## Not in this Specification

- No automatic fixes, watch mode or editor integration.
- No writes to the repository during a lint run.
- No telemetry, and no network use except the configured backend for judged rules.
- No built-in rule sets, local model backend, Windows build, homebrew-core formula, crates.io package or notarization.
- No rule sets with `conversation`, `payload` or `change_set` inputs in `lint`; those belong to later commands.

## Proof obligations

| ID | For | Passes when | Fails when |
| --- | --- | --- | --- |
| V1 | R1, R3 | A config with one mechanical and one judged rule set lints a fixture repository, and every format lists the same findings with rule, file, location, severity and message | a finding is missing, or formats disagree |
| V2 | R2 | Fixtures produce each exit code 0–4, and a run with both an error and an incomplete part exits 1 and lists both | any outcome maps to a different code |
| V3 | R4 | `--changed` and `--since` select exactly the files changed in a scripted Git history | an unchanged file is linted, or a changed one is missed |
| V4 | R5 | `--check` makes no model request, catches each invalid-config case, and `--init` refuses to overwrite | a model request is made, or an invalid config passes |
| V5 | R6 | The engine's existing protocol tests pass through `sunstack exec` with only the severity words changed | any other protocol change |
| V6 | R7 | `npx sunstack lint --format json` and the Rust library call return the same report as the CLI for the same fixture | reports differ |
| V7 | R8, R9 | Discovery picks the nearest config; each invalid config exits 2 with file, line and column | the wrong config is used, or an error lacks its location |
| V8 | R10 | A table of severity cases (default, `rules`, wildcard, exact, overrides in order, `off`) gives the expected severity for each file | any case differs |
| V9 | R11, R12 | A non-document rule set is refused by `--check`; ignored files are never read | either is not the case |
| V10 | R13, R17 | A mechanical-only config runs with no key; a judged config without a key exits 2 before any request; a real Jev judgment completes from a static Linux build with no CA bundle | any is not the case |
| V11 | R14 | Rule files with `block` or `nudge` are refused with the replacement named; reports use `error` and `warn` | old words are accepted or appear |
| V12 | R15 | The published schema rejects and accepts the same configs as the binary | they disagree |
| V13 | R16, R18 | Each release build loads only the allowed libraries, and runs `--version`, `lint --check` and a lint run on clean macOS, Alpine and Debian | an extra library is loaded, or a run fails |
| V14 | R19–R23 | In a private test repository, a test release produces the archives and checksums, then `brew install`, the shell installer and `npm i` each install a binary that reports the release version and lints a fixture | any channel fails or reports another version |
