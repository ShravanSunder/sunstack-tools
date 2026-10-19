# sunstack runner: Verification Design

How each requirement will be shown to hold. This is a plan: nothing in it has been observed. Requirements: [requirements.md](requirements.md). Obligations, references and proof obligations: [specification.md](specification.md). Seams: [program-design.md](program-design.md#proof-seams).

## Proven by

| Capability | Requirement | Proven by | Stays unverified |
| --- | --- | --- | --- |
| runner | U1 Lint a repository and know from the exit code | Run `sunstack lint` on fixture repositories built to hit every outcome, and read each report and exit code. | Repositories larger than the fixtures |
| runner | U2 The same lint from Node and Rust | Install the npm package and call the Rust library on the same fixture, and compare their reports with the CLI's. | A TypeScript API (out of scope) |
| runner | U3 One binary for the whole system | Run the engine's existing protocol tests through `sunstack exec`, and check that `sunstack lint` and `sunstack exec` are the same installed binary. | Commands for later uses, which don't exist yet |
| config | U4 One severity vocabulary | Load rule files using the new words and the old ones, run them, and read every report and schema for the old words. | — |
| config | U5 A config of rule sets and severities | Lint fixtures across a table of severity settings and invalid configs, and check `--check`, `--rules` and `--init`. | Built-in rule sets (out of scope) |
| config | U6 `sunstack.lint.json` points at rule sets | Discover configs from nested folders, and confirm configs never contain rules. | — |
| config | U7 Jev now, room for a local model | Lint with and without a key, and run one real Jev judgment. | A local model backend (out of scope) |
| portable-builds | U8 Runs on Linux and macOS with nothing installed | Run each release build in clean Alpine and Debian containers, x64 and arm64, and natively on macOS, and inspect what each loads. | Intel Mac hardware (the x64 build runs under Rosetta here); Linux distributions other than the two; private CAs |
| portable-builds | U9 git2-rs with libgit2 compiled in | Inspect each build's linked libraries. | — |
| install | U10 Install the usual way | Run the generated release in a private test repository, then install those exact files through Homebrew, the shell installer and npm. | Installing from the public release addresses, until the first public release |
| install | U11 Easy to ship and run everywhere | Same release rehearsal: one tag produces every channel at one version. | Public publishing (not done in this work) |

## Coverage rows

| Requirement | Obligations | Reference | Target and journey | Lifetime | Passes when | Reuses | Unverified |
| --- | --- | --- | --- | --- | --- | --- | --- |
| U1 | R1, R2, R4 | Spec C2 exit table; the fixture's expected findings (rule, file, line, severity) written beside it | macOS arm64 dev build and the Linux musl release build; `needed: verify-sunstack-rule lint-runner journey`, entering at `sunstack lint`, reading stdout, stderr and the exit code ([S1](#s1-lint-fixture-repositories)) | lasting | Each fixture's report lists exactly its expected findings and exits with its expected code; error plus incomplete exits 3; nothing selected exits 0 with "No checks selected"; the default scope in a scripted history lints only changed and dependency-affected targets. A build that exits 0 on an error finding, or lints an unchanged unaffected file, fails | none | Large repositories; Windows |
| U2 | R3, R7 | The CLI's own `json` report for the same fixture | Node from the npm package installed in a clean `node` container, and the `sunstack-lint` library called from a Rust test ([S1](#s1-lint-fixture-repositories), [S4](#s4-release-rehearsal)) | lasting | `npx sunstack lint --format json` and the library return reports equal to the CLI's, field for field; `default`, `github` and `sarif` list the same findings. A wrapper that drops findings or alters the exit code fails | none | TypeScript API (out of scope) |
| U3 | R6 | The engine's existing protocol tests and their expected responses | `sunstack exec` from the dev build; the existing process suite under `crates/sunstack-rule-cli/tests/` (to move with the binary) | lasting | The existing `process_*` tests pass against `sunstack exec` with only `block`/`nudge` replaced by `error`/`warn` in their fixtures and expectations. A build that changes any other protocol byte fails | the existing process suite, which drives the real binary over stdin/stdout | — |
| U4 | R14 | Spec R14; the renamed `Severity` and `Activation` words | dev build; S1 plus the engine's verdict tests in `sunstack-rules` | lasting | Rule files with `error`/`warn` load; `block` or `nudge` is refused, naming the replacement; no report, schema or `exec` response contains the old words. A build that accepts `block` fails | engine verdict-evaluation tests, renamed | — |
| U5 | R5, R9, R10, R11, R15 | Spec R10 precedence and band inequalities; a table of severity cases with the expected severity per file | dev build; S1; engine policy tests | lasting | Every severity case gives its expected severity; a rule `off` for every target makes no model request (recording backend); each invalid config exits 2 with file, line and column; `--check` makes no request and reads no target; the published schema accepts and rejects the same configs as the binary. A build that pays for an `off` rule, or applies overrides out of order, fails | engine recording-backend test pattern | Built-in rule sets |
| U6 | R8, R12 | Spec R8, R12 | dev build; S1 | lasting | The nearest config wins from nested folders; `--config` overrides; with none, exit 2 names the file sought; ignored files are never targets, and a declared-context ignored file still reaches its rule. A build that reads a parent config past a nearer one fails | none | — |
| U7 | R13, R17 | Spec R13; an OpenRouter judgment of a fixture with a known failing answer | Linux arm64 musl release build in a clean Alpine container with `/etc/ssl` hidden, given the key from the owner's 1Password item for that command only ([S3](#s3-hosted-judgment)) | lasting | A config with no reachable model work runs with no key; reachable model work without a key exits 2 before any request; with the key, the judged rule returns its expected verdict from the static build. A build that sends a request without a key, or needs the system CA bundle, fails | engine hosted-rules route (`references/hosted-decisions.md`), retargeted at the static build | Private CAs; other models |
| U8 | R16 | Spec R16: only operating-system libraries | each release build; `needed: container-matrix journey` ([S2](#s2-clean-container-matrix)) | lasting | Linux builds have no interpreter and no needed libraries; macOS builds load only `/usr/lib` and `/System` libraries; each runs `--version`, `lint --check` and a lint run with the expected exit code in clean Alpine and Debian (x64, arm64) and on macOS. A build linking Homebrew libgit2 fails | the earlier distribution experiment's ELF and `otool` method | Intel Mac hardware; other distributions |
| U9 | R18 | Spec R18 | each release build; S2 | lasting | No build lists libgit2 as a separate library; Cargo features show `vendored-libgit2` and no Git network features | — | — |
| U10 | R19, R20, R21, R22 | Spec R19–R22; the release workflow's own checksums | a private test repository with the generated workflow, then a local mirror serving those exact release files ([S4](#s4-release-rehearsal)) | lasting | One test tag produces four archives with checksums, the installer, a formula with real checksums and the npm package; installing through Homebrew (macOS and Linux), the shell installer and npm gives a binary that lints a fixture with the right exit codes and stops on Ctrl-C, including through the npm launcher. A channel that installs other bytes, or loses the exit code or the signal, fails | none | Public release addresses, until the first public release |
| U11 | R23 | The test tag's version | S4 | lasting | `sunstack --version` from every channel equals the tag, and the formula and npm versions match it. A channel reporting another version fails | none | Public publishing |

## Shared observations

### S1 lint fixture repositories

Small Git repositories built by a script, each with a `sunstack.lint.json`, rule sets and target files, plus an expected-results file beside each. Together they cover:
- every exit code;
- every severity case;
- invalid configs;
- the default, `--since` and `--all` scopes in a scripted history, including a changed rule and a changed config;
- ignored and context files;
- and all four formats.

They run against the dev build and the release builds, with an unreachable backend endpoint for every fixture that must make no model request. Lifetime: lasting, as `verify-sunstack-rule`'s lint-runner journey. Serves U1, U2, U4, U5, U6.

### S2 clean-container matrix

Each release build runs in fresh `alpine` and `debian:stable-slim` containers pinned by digest, x64 and arm64, read-only and without network. Each run does `--version`, `lint --check` and one fixture lint, and inspects the binary's linkage. The macOS builds run natively, with the x64 build under Rosetta. Lifetime: lasting, rerun every release. Serves U8, U9.

### S3 hosted judgment

One judged rule over a fixture whose correct verdict is known to fail. It runs from the static Linux arm64 build in Alpine with the CA bundle hidden. The key is supplied to that one command from the owner's 1Password item and is never printed. Lifetime: lasting, run when the backend or the build changes. Serves U7.

### S4 release rehearsal

A private test repository holds a copy of `sunstack-tools` with the generated cargo-dist workflow, plus a private test tap. Pushing a test tag runs the real workflow. Its release files are downloaded with an authenticated `gh release download`, then served from a local mirror so the installers, which cannot authenticate, fetch those exact bytes. Install through Homebrew (macOS and in a Linux container), the shell installer, and npm in a clean `node` container. Lifetime: lasting, the pre-release check for each version. Serves U2, U10, U11.

## Unverified at delivery

- Installing from the public GitHub, Homebrew tap and npm addresses. These exist only after the first public release, which is not part of this work.
- Intel Mac hardware. The x64 macOS build runs under Rosetta on Apple silicon here.
- Linux distributions other than Alpine and Debian, private CA certificates, and Windows.
