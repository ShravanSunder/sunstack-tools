# @sunstack/sdk

Typed TypeScript access to the Sunstack engine. Load an existing `sunstack.lint.json`
project or author a complete configuration as TypeScript values, check it without
inference, run it against a repository, and read typed results. The Rust engine
(the same one the `sunstack` CLI uses) decides validity, executes rules and returns
every outcome; the declarations are generated from its Rust contracts.

## Install

```sh
npm install @sunstack/sdk
```

Requirements: Node 26 or later on macOS (arm64, x64) or Linux (arm64, x64; glibc or
musl). The package manager installs the matching prebuilt native addon
(`@sunstack/sdk-<platform>`); no Rust toolchain is needed. Zod is not required.

Type checking is a separate step from running: `node file.ts` strips types and does
not check them. Run `tsc --noEmit` (TypeScript 7) to check a consumer.

## Author a configuration in TypeScript

Every definition is a typed value: rulebook Markdown, marker-linked rule entries,
ast-grep matchers and utilities, evidence kinds, filters, questions, decision
graphs, model settings and limits. No definition files are written or read.

```ts
import { check, run } from '@sunstack/sdk';
import type { Configuration } from '@sunstack/sdk';

const configuration = {
	id: 'instruction-standards',
	base_directory: import.meta.dirname,
	marker_prefix: 'K',
	check: { files: ['skills/**/SKILL.md'] },
	decision_model: { model: 'jev-latest' },
	limits: { max_file_bytes: 1_048_576n }, // u64 limits are bigint
	evidence: {
		kind: 'inline',
		utilities: {},
		matchers: { paragraph: { language: 'Markdown', rule: { kind: 'paragraph' } } },
		kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
	},
	rulebooks: [
		{
			kind: 'inline',
			id: 'ownership-standards',
			markdown: '# Standards\n\nEach instruction names its responsible actor. <!-- K001 -->\n',
			entries: {
				K001: {
					name: 'instruction_actor',
					evidence: 'paragraph',
					severity: 'warn',
					questions: {
						actor: {
							type: 'noul',
							instructions: { check: 'body', against: 'standard', task: 'Is the actor named?' },
						},
					},
					decision: {
						start: 'actor',
						steps: {
							actor: {
								ask: 'actor',
								yes_at_or_above: 0.8,
								no_at_or_below: 0.2,
								yes: 'pass',
								no: { finding: 'Name the actor.', at: 'body' },
								uncertain: 'uncertain',
							},
						},
					},
				},
			},
		},
	],
} satisfies Configuration;

const admission = await check(configuration); // no credential, provider or checked file
const outcome = await run(configuration, { root: '/abs/path/to/repo', scope: { kind: 'all' } });
```

`examples/inline-configuration.ts` is the complete version with a filter, a choice
question and a gated rule.

## Load and edit an existing configuration

`load` uses the CLI's discovery from an absolute directory, reads every referenced
definition once, and returns frozen, editable typed values with their origins. A
changed copy executes the change and reports it as `edited`; the loaded value and
the files on disk are untouched. Calls never reread a loaded definition.

```ts
import { load, run } from '@sunstack/sdk';
import type { Configuration } from '@sunstack/sdk';

const loaded = await load({ directory: '/abs/path/to/project' });
if (loaded.kind === 'loaded') {
	const changed = {
		...loaded.value.configuration,
		limits: { ...loaded.value.configuration.limits, max_files: 100 },
	} satisfies Configuration;
	const outcome = await run(changed, { root: '/abs/path/to/repo', scope: { kind: 'all' } });
}
```

See `examples/load-and-edit.ts`.

## Document conventions

HTML and Markdown conventions are configuration, set on any evidence form (`inline`,
`captured` or `project`) next to `kinds`. With none set, the standard defaults apply:
`<template>` contents stay out of HTML section outlines, `alt` and `aria-label` are
read as asking text, and Markdown heading fragments are not resolved.

```ts
import type { Configuration, HtmlEvidenceSettings, MarkdownEvidenceSettings } from '@sunstack/sdk';

// The 0.14.1 conventions, restored explicitly.
const restored = {
	html: { skip_tags: ['ex-markdown'], text_attributes: { 'ex-mermaid': ['label'] } },
	markdown: { heading_anchors: 'github' },
} as const satisfies { html: HtmlEvidenceSettings; markdown: MarkdownEvidenceSettings };

const configured = {
	...configuration,
	evidence: { ...configuration.evidence, ...restored },
} satisfies Configuration;
```

- `html.skip_tags` leaves headings inside the named tags out of the section outline;
  their text is still asking text.
- `html.text_attributes` reads the listed attributes of a tag as `[name: value]`
  tokens of asking text, in source order. Raw fields, spans and locations stay the
  source.
- `markdown.heading_anchors: 'github'` resolves heading fragments with GitHub's slugs.
  Without it, a heading fragment is `{ status: 'unsupported', reason:
  'heading_scheme_not_configured' }`. Explicit HTML `id` and anchor `name` values
  resolve either way. A fragment into another kind of document is unsupported with
  reason `unsupported_document`.

The conventions are root settings. A loaded project whose conventions are unchanged
keeps its file origin; an edited copy reports the root as `edited`. A change on disk
takes effect only when you call `load` again. Blank names, unknown keys and unknown
schemes are refused by the engine.

## Section heading fields

Native section kinds (`html_units`, or `markdown_units` with units exactly `['section']`)
can read a section's own heading and the headings that enclose it:

```ts
import type { FieldSource, FieldValue } from '@sunstack/sdk';

const headingFields = {
	heading: { from: 'section', text: 'heading' }, // one text
	heading_path: { from: 'enclosing', kind: 'outline', text: 'heading' }, // outermost first
} as const satisfies Readonly<Record<string, FieldSource>>;

function texts(value: FieldValue | undefined): readonly string[] {
	if (value?.state === 'text') return [value.value];
	if (value?.state === 'texts') return value.values;
	return [];
}
```

- **HTML:** a heading is the h1–h6 element's text, with entities decoded once:
  `&lt;Widget&gt;` reads `<Widget>`.
- **Markdown:** a heading keeps its inline source and drops only the ATX `#` markers or
  the setext underline.
- **Values and spans:** stored values and the asking view hold the same heading text,
  and spans point at the original heading.
- **Enclosing kind:** the kind it names (`outline` above) must itself be a native
  section kind. The engine refuses heading text from other kinds, from `same_file`,
  and through a target's `as_kind`.

`examples/heading-fields.ts` runs this without a model.

## Typed outcomes

No call throws for a domain or detectable native ending. Narrow on `kind`:

| Call    | Outcomes                                                                                                                                   |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `load`  | `loaded`, `invalid`, `unavailable` (a definition could not be read), `failed`                                                              |
| `check` | `valid`, `invalid`, `unavailable`, `cancelled`, `failed`                                                                                   |
| `run`   | `completed` (status `clean`, `nothing_selected`, `findings` or `uncertain`), `invalid`, `unavailable`, `incomplete`, `cancelled`, `failed` |

`failed` is an adapter failure: `before_dispatch` has no effects (unsupported runtime
or platform, missing addon, contract mismatch, unencodable input);
`after_dispatch` means effects are unknown and the SDK does not retry. A native task
that stops unexpectedly during a run is `failed` after dispatch, never `incomplete`;
its report holds the progress made so far, or is absent if nothing had run. Diagnostics
keep the definition `origin` separate from the checked input `path` that stopped a run.
`json_path` comes in two forms:

- **Decoder refusals:** when the value itself does not decode, `json_path` lists one
  segment per key, for example `['limits', 'max_file_bytes']`.
- **Admission refusals:** for a decoded value the engine refuses, it holds the engine's
  rendered path as one string, for example `['check.files']`.
- **Rule-file refusals:** these name the refused field when the decoder may disclose it,
  for example `['only_when']`.

```ts
if (outcome.kind === 'completed' && outcome.report.status.kind === 'findings') {
	for (const finding of outcome.report.findings)
		console.log(finding.file, finding.line, finding.message);
}
```

Integers follow their Rust types and always cross exactly:

- **Engine full-width fields** (u64 and usize, such as `max_file_bytes` or result
  line numbers) are `bigint`.
- **ast-grep positions** follow the pinned upstream schema: a safe integer stays a
  `number`, and a wider value is a `bigint`. Pass a `bigint` when you need an exact
  large position.
- **Structured JSON values**, such as rule references, keep finite floats as
  `number` and integers exactly, so read them as `number | bigint`.

Unsafe `number` integers, non-finite numbers, functions, cycles and other non-data
values are refused before dispatch.

## Run options

- `root` is an absolute checked directory, used exactly as `sunstack lint --root`.
  File globs, filter folders and named `paths` resolve against it. It may differ
  from the configuration's `base_directory`, which resolves definition references only.
- `scope`: `{ kind: 'default' }` (staged, unstaged and untracked changes; needs Git),
  `{ kind: 'since', rev }` or `{ kind: 'all' }`.
- `signal`: an `AbortSignal` that cancels only this call. An already-aborted signal
  starts no work.

`run` uses the existing credential route (`OPENROUTER_API_KEY`, then the OS
credential store). The SDK never changes the process working directory or
environment.

## Validation without Zod

The generated declarations constrain keys, variants, outcomes and result access at
compile time; the Rust engine decides refinements (nonzero limits, globs, references,
marker binding, patterns). Zod or any other validator is optional and not shipped.

## Developing this package

From the repository root:

```sh
pnpm --filter @sunstack/sdk generate        # regenerate declarations from Rust
pnpm --filter @sunstack/sdk build:native    # build this host's addon
pnpm --filter @sunstack/sdk build:typescript
pnpm --filter @sunstack/sdk typecheck
pnpm --filter @sunstack/sdk lint
pnpm --filter @sunstack/sdk test
pnpm --filter @sunstack/sdk pack:candidate  # pack the SDK and this host's platform package
```
