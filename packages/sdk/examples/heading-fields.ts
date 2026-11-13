// Native section heading fields: each Markdown section's own heading and its enclosing
// headings, outermost first. A mechanical rule needs no model, so no credential is used.
//
//   node examples/heading-fields.ts <absolute-checked-root>
import { check, run } from '@sunstack/sdk';
import type { Configuration, FieldSource, FieldValue } from '@sunstack/sdk';

const [root = process.cwd()] = process.argv.slice(2);

const sections = { mode: 'unit', units: ['section'] } as const;
const headingFields = {
	heading: { from: 'section', text: 'heading' },
	heading_path: { from: 'enclosing', kind: 'outline', text: 'heading' },
} as const satisfies Readonly<Record<string, FieldSource>>;

const configuration = {
	id: 'heading-fields',
	base_directory: import.meta.dirname,
	marker_prefix: 'K',
	check: { files: ['docs/*.md'] },
	decision_model: { model: 'jev-latest' },
	evidence: {
		kind: 'inline',
		utilities: {},
		matchers: {},
		kinds: {
			// The referenced kind for enclosing headings must itself be a native section kind.
			outline: { markdown_units: sections, fields: {} },
			section: { markdown_units: sections, fields: headingFields },
		},
	},
	rulebooks: [
		{
			kind: 'inline',
			id: 'heading-rules',
			markdown: 'Every section has a heading. <!-- K001 -->\n',
			entries: {
				K001: {
					name: 'has_heading',
					evidence: 'section',
					severity: 'warn',
					decision: {
						start: 'present',
						steps: {
							present: {
								mechanical_check: { tool: 'complete', field: 'heading' },
								yes: 'pass',
								no: { finding: 'Give the section a heading.' },
							},
						},
					},
				},
			},
		},
	],
} satisfies Configuration;

/** One heading, ordered enclosing headings, or nothing. */
function texts(value: FieldValue | undefined): readonly string[] {
	if (value?.state === 'text') return [value.value];
	if (value?.state === 'texts') return value.values;
	return [];
}

const admission = await check(configuration);
process.stdout.write(`check: ${admission.kind}\n`);
const outcome = await run(configuration, { root, scope: { kind: 'all' } });
if (outcome.kind !== 'completed') {
	process.stdout.write(`run: ${outcome.kind}\n`);
	process.exit(1);
}
process.stdout.write(`run: ${outcome.report.status.kind}\n`);
for (const { piece } of Object.values(outcome.report.piece_evidence)) {
	if (piece.kind !== 'section') continue;
	const [heading = ''] = texts(piece.fields['heading']);
	const path = texts(piece.fields['heading_path']).join(' > ');
	process.stdout.write(`${piece.where.file}: ${heading} | ${path}\n`);
}
