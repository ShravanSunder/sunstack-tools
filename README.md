# Sunstack tools

Demistify is a visual-document experiment using native Web Components. Agents write HTML composition and Markdown/Mermaid content; the reusable library owns rendering, design, icons and interaction.

- `packages/demistify`: drop-in browser runtime, shared theme, optional content packaging.
- `apps/demistify-poc`: five independent presentation experiments, comparison report and composition/layout showcases.
- `docs/wip/demystify`: detailed continuation handoff and source styling study.

For a plain HTML explainer, use the public experimental v1 assets:

```html
<link rel="stylesheet" href="https://demistify.getagentstudio.dev/v1/theme.css" />
<script type="module" src="https://demistify.getagentstudio.dev/v1/runtime.js"></script>
<ex-document>
	<ex-markdown
		><script type="text/markdown">
			# Session architecture

			Sessions survive transport reconnections.
		</script></ex-markdown
	>
</ex-document>
```

Give the page ordinary HTML document metadata and a title. Read the [matching authoring guide](https://demistify.getagentstudio.dev/v1/authoring/SKILL.md) for components and relative references. Inline or embedded content needs no server; referenced Markdown/JSON files need HTTP serving or the supplied embedding/standalone tools. CDN links need network access. Experimental v1 may have breaking changes until the owner declares stable major compatibility.

Library changes use a Changeset, then a separate version PR and protected main deployment. See the [release instructions](packages/demistify/release/README.md) and [v1 deployment handoff](docs/wip/demystify/2026-10-16-v1-deployment-handoff.md).

From the repository root, with Node 26 and pnpm 12:

```sh
pnpm install
pnpm check
pnpm build
pnpm --filter @sunstack/demistify-poc preview --port 4174
```

Open the local preview to compare all five independent pages. A library consumer needs only the built `runtime.js`, `theme.css` and their shared companion assets; the demo host is optional. See [the library reference](packages/demistify/README.md) and [the iteration handoff](docs/wip/demystify/2026-10-14-demistify-handoff.md).

The pre-existing rule-engine design documents remain under `docs/specs/` and are unrelated to this experiment.

## Authoring measurements

Six exploratory Sol/Luna runs compared Demistify with ordinary HTML/CSS artifacts. Demistify produced smaller reader source in each matched pair; author-session usage and first-handback quality varied. See the [measured counts, browser findings and limitations](docs/benchmarks/demistify-authoring-2026-10-17.md). Source size alone does not establish billed savings or reliability.

## Sunstack runner

`sunstack lint` checks repository documents using marked rulebooks and decision graphs. The same lint is
available as the `sunstack-lint` Rust library. `sunstack exec` exposes the native JSON protocol.
Build from this repository with the pinned Rust toolchain:

```sh
cargo run -p sunstack -- --version
cargo run -p sunstack -- lint --init
cargo run -p sunstack -- lint --check
cargo run -p sunstack -- lint --all --format json
cargo run -p sunstack -- exec < request.json
```

See [the executable protocol](crates/sunstack/README.md) for request framing, native operations and exit codes.

### Install a release

The release workflow builds macOS arm64/x64 and static Linux arm64/x64 archives,
checksums, a shell installer, a Homebrew formula, an npm package and the config schema.
Public release publication is a separate step; the commands below require a published release.
The formula is pushed to `ShravanSunder/homebrew-taps` with a write deploy key whose
private half is the `HOMEBREW_TAP_DEPLOY_KEY` release secret. The npm package
`@sunstack/cli` is published through npm trusted publishing (GitHub OIDC), so no npm
token is stored. The npm publish job runs only when the repository variable
`NPM_TRUSTED_PUBLISHING` is `true`; set it once the package's trusted publisher is
configured on npmjs.com. Until then, releases publish every other channel and skip npm.
The plain name `sunstack` on npm is a deprecated placeholder that points to
`@sunstack/cli`.

The [release workflow](.github/workflows/release.yml) starts from cargo-dist 0.32.0's
generated workflow and is maintained by hand (`allow-dirty = ["ci"]`). When upgrading
cargo-dist, re-diff its generated workflow and retain the archive linkage and install
verification steps.
The `fresh-install-smoke` job runs after publishing; it detects installation and
runtime failures after the release assets are available.

```sh
brew install shravansunder/taps/sunstack
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ShravanSunder/sunstack-tools/releases/latest/download/sunstack-installer.sh | sh
# Pin a version by downloading that release's installer:
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ShravanSunder/sunstack-tools/releases/download/v0.6.0/sunstack-installer.sh | sh
npm i -D @sunstack/cli
node_modules/.bin/sunstack --version
```

Archives use `.tar.gz`, and extract with standard `tar` and `gzip`. Compare their
SHA256s with the release checksums. The npm package downloads the matching archive
from GitHub during installation and writes the executable inside the installed
package. This requires network access; pnpm may require `pnpm approve-builds`.
It has no TypeScript API.

From Node, any of these runs the same release build:

```sh
npm i -D @sunstack/cli && node_modules/.bin/sunstack lint   # or an npm script
npm i -g @sunstack/cli && sunstack lint
npx @sunstack/cli lint
pnpm dlx @sunstack/cli lint
```

Each route passes on the exit code (0–3) and stdout, the report included, unchanged.
npm, npx and pnpm may add their own lines to stderr, such as install messages. When
you need stdout exact from an npm script, run it as `npm run --silent lint`, because
a plain `npm run` first prints its own header. In npm scripts, write `exec` so that
stopping npm stops the run:

```json
{ "scripts": { "lint": "exec sunstack lint" } }
```

Stopping a run depends on what sits in front of the bin. "4" means the run cancels,
prints its full report and exits 4. "Not passed on" means the stop never reaches the
run, and the run may keep going, so signal the whole process group instead.

| Route                                                | Ctrl-C at a terminal                    | SIGINT or SIGTERM to that process only | SIGHUP to that process only |
| ---------------------------------------------------- | --------------------------------------- | -------------------------------------- | --------------------------- |
| The bin run directly, or `sunstack` after `npm i -g` | 4                                       | 4                                      | the run ends by SIGHUP      |
| npm script written `exec sunstack …`                 | 4                                       | 4                                      | not passed on               |
| npm script without `exec`, on Linux                  | the run cancels; npm reports the signal | not passed on                          | not passed on               |
| `pnpm dlx @sunstack/cli …`                           | 4                                       | 4                                      | the run ends                |
| `npx @sunstack/cli …` on macOS                       | 4                                       | 4                                      | not passed on               |
| `npx @sunstack/cli …` on Linux                       | the run cancels; npx exits 130, not 4   | not passed on                          | not passed on               |
| `npx --package=@sunstack/cli -c 'exec sunstack …'`   | 4                                       | 4                                      | not passed on               |

For tools and CI that stop runs by signal, use the bin, an `exec` script, `pnpm dlx`,
or write npx with `-c 'exec …'` as in the last row. On Linux, plain `npx` runs the bin
through Debian's `sh`, which stays in the middle and keeps the stop from reaching it.

GitHub Actions cancels a step by sending SIGINT, then SIGTERM, to the step's own shell
([GitHub's cancellation reference](https://docs.github.com/en/actions/reference/workflow-cancellation-reference)).
To let a cancelled job print its cancelled report, make `sunstack` the step's last command
with `exec`:

```yaml
- run: exec sunstack lint
# or through npm
- run: exec npx --package=@sunstack/cli -c 'exec sunstack lint'
```

### Configure and run lint

Run `sunstack lint --init`, then add the referenced rulebook, rule entries and
ast-grep evidence project. See the [CLI configuration guide](crates/sunstack/README.md#repository-lint)
for the current configuration shape, scope flags, `--check`, `--evidence` and report
formats. Version 0.2.0 refuses the former `rule_sets` configuration. Kinds can also
use model-based `select` and `within` containment; selection outcomes and per-file
counts are visible in reports and `--evidence` listings. Selected kinds require a
credential for evidence listings as well as ordinary runs.

Exit codes are 0 for clean or nothing selected, 1 for failing findings, 2 for
invalid input, 3 for incomplete work, 4 for cancellation and 5 for uncertain
decisions when no higher-priority result applies.

### Privacy, cost and limits

Mechanical checks run locally. Evidence selection sends bounded candidate and
shared field views; asking rules send a piece's bounded field view to the
configured backend. Such work needs
`decision_model: { "model": "jev-latest" }` and an OpenRouter key
from `OPENROUTER_API_KEY` or the OS credential store. The default endpoint is `https://openrouter.ai/api/v1/systemone`.
The environment wins without touching the store. To save the current environment
key to macOS Keychain or Linux Secret Service, run `sunstack credentials save`;
ordinary model runs never save it automatically. The command is listed in `sunstack --help`.
Set `SUNSTACK_CREDENTIAL_STORE=off` to disable store lookup and saving, for example
in CI. Credentials are never included in reports.
Judged rules cost money; change scopes and `limits` bound that work. Reports
include requests, tokens and provider-reported cost; missing cost stays absent.

Default limits are 2,000 files, 1 MiB per file, 64 MiB total, 10,000 matches, 8 MiB of
transcript, 200 model requests, 32,000 input tokens and 256 KiB per request, 32 questions
per request, 60 seconds per attempt, 2 retries with 0.5–8 second backoff, 1 MiB per
response and 4 concurrent requests. Speculation is off. The schema lists configurable
limits, and every JSON report records the effective values.

Linux executables load no shared libraries; macOS executables load only OS libraries.
Git uses vendored libgit2 with Git network features disabled. HTTPS uses compiled-in
public CA roots and works without a system CA bundle. Private CAs in the operating
system trust store are not used.
