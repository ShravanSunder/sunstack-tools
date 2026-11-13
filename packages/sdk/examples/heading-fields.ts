import { inspect as formatValue } from 'node:util';

// Native section heading fields: each HTML and Markdown section's own heading, its enclosing
// headings (outermost first) and the source bytes of its heading. A mechanical rule needs no
// model, so no credential is used.
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

const hasHeading = {
	start: 'present',
	steps: {
		present: {
			mechanical_check: { tool: 'complete', field: 'heading' },
			yes: 'pass',
			no: { finding: 'Give the section a heading.' },
		},
	},
} as const;

const configuration = {
	id: 'heading-fields',
	base_directory: import.meta.dirname,
	marker_prefix: 'K',
	check: { files: ['docs/*.md', 'docs/*.html'] },
	decision_model: { model: 'jev-latest' },
	evidence: {
		kind: 'inline',
		utilities: {},
		matchers: {},
		kinds: {
			// The referenced kind for enclosing headings must itself be a native section kind.
			outline: { markdown_units: sections, fields: {} },
			section: { markdown_units: sections, fields: headingFields },
			html_outline: { html_units: sections, fields: {} },
			html_section: {
				html_units: sections,
				fields: {
					...headingFields,
					heading_path: { ...headingFields.heading_path, kind: 'html_outline' },
				},
			},
		},
	},
	rulebooks: [
		{
			kind: 'inline',
			id: 'heading-rules',
			markdown:
				'Every Markdown section has a heading. <!-- K001 -->\n\nEvery HTML section has a heading. <!-- K002 -->\n',
			entries: {
				K001: { name: 'has_heading', evidence: 'section', severity: 'warn', decision: hasHeading },
				K002: {
					name: 'has_html_heading',
					evidence: 'html_section',
					severity: 'warn',
					decision: hasHeading,
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
if (admission.kind !== 'valid') {
	process.stderr.write(`${formatValue(admission, { depth: null })}\n`);
	process.exit(1);
}
const outcome = await run(configuration, { root, scope: { kind: 'all' } });
if (outcome.kind !== 'completed') {
	process.stdout.write(`run: ${outcome.kind}\n`);
	process.stderr.write(`${formatValue(outcome, { depth: null })}\n`);
	process.exit(1);
}
process.stdout.write(`run: ${outcome.report.status.kind}\n`);
for (const { piece } of Object.values(outcome.report.piece_evidence)) {
	if (piece.kind !== 'section' && piece.kind !== 'html_section') continue;
	const heading = piece.fields['heading'];
	const [text = ''] = texts(heading);
	const path = texts(piece.fields['heading_path']).join(' > ');
	// Spans are UTF-8 byte offsets of the original heading in the checked file.
	const bytes =
		heading?.state === 'text'
			? heading.spans.map((span) => `${span.byte_start}-${span.byte_end}`).join(',')
			: '';
	process.stdout.write(`${piece.where.file}: ${text} | ${path} @ ${bytes}\n`);
}
// Supply nonempty docs/*.md or docs/*.html whose sections all have headings.
process.exitCode =
	outcome.report.status.kind === 'clean' && outcome.report.results.length > 0 ? 0 : 1;
