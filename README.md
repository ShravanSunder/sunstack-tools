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

`sunstack lint` checks repository documents using authored rule sets. The same lint is
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

```sh
brew install shravansunder/tap/sunstack
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ShravanSunder/sunstack-tools/releases/latest/download/sunstack-installer.sh | sh
# Pin a version by downloading that release's installer:
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ShravanSunder/sunstack-tools/releases/download/v0.1.0/sunstack-installer.sh | sh
npm i -D sunstack
npx sunstack --version
```

Archives use `.tar.gz`, and extract with standard `tar` and `gzip`. Compare their
SHA256s with the release checksums. The npm package downloads the matching archive
from GitHub during installation and writes the executable inside the installed
package. This requires network access; pnpm may require `pnpm approve-builds`.
The package launches the command, passes its exit code through and forwards signals.
It has no TypeScript API.

### Configure and run lint

Run `sunstack lint --init`, then add at least one rule-set entry. The skeleton has no
built-in rules and is refused until configured. Each rule set must declare exactly one
`document` input. Paths and file globs in the config are relative to its directory.

```json
{
	"$schema": "https://github.com/ShravanSunder/sunstack-tools/releases/download/v0.1.0/sunstack.lint.schema.json",
	"rule_sets": {
		"docs": { "path": "rules/docs.ruleset.json", "files": ["docs/**/*.md"] }
	},
	"rules": { "docs/*": "error" }
}
```

The command finds the nearest `sunstack.lint.json` in the current directory or its
parents; `--config <file>` selects one explicitly. `--check` validates config and rule
sets without reading target content or needing a backend. `--rules` lists every rule
as JSON; `--rules <file>` adds that file's effective setting and its origin.

With no paths, lint checks staged, unstaged and untracked changes in Git. Named paths
default to full scope. Use `--all` for every configured target (also outside Git), or
`--since <rev>` for changes since a revision, including worktree edits. A config change
widens the run to full scope and is reported. Git-ignored files and `ignore_patterns`
are excluded. Severity precedence is authored, entry wildcard, exact rule, then matching
override blocks in order. An exact rule wins within a block; `off` prevents model work.

Formats are `default`, `json`, `github` and SARIF 2.1.0 (`--format sarif`). Exit codes:
0 clean or no checks selected; 1 error findings or warnings above `--max-warnings`;
2 invalid input/config/backend; 3 incomplete; 4 cancelled. Incomplete and cancelled
runs retain completed findings. Warnings alone do not fail unless a warning limit is set.

### Privacy, cost and limits

Mechanical rules send no text over the network. Selected text for judged rules and
model-based applicability decisions is sent to the configured backend. Such work needs
`backend: { "kind": "jev_openrouter", "model": "jev-latest" }` and `OPENROUTER_API_KEY`
in the environment. The default endpoint is `https://openrouter.ai/api/v1/systemone`.
The key is read only when model work is reachable and is never included in reports.
Judged rules cost money; change scopes, `off` and `limits` bound that work. Reports
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
