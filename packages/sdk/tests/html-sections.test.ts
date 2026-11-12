// Native HTML heading sections authored inline and executed through the addon: section
// field text, the absent parent of a top-level section, asking projection versus raw
// mechanical text, and selector refusals.
import assert from 'node:assert/strict';
import { test } from 'node:test';

import type {
	CheckOutcome,
	Configuration,
	EvidenceKindDefinition,
	SdkReport,
} from '../src/index.ts';
import { check, run } from '../src/index.ts';
import { isCheckOutcome } from '../src/reply-decoding.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

const sectionKind: EvidenceKindDefinition = {
	html_units: { mode: 'unit', units: ['section'] },
	fields: {
		complete: { from: 'section', text: 'full' },
		own_body: { from: 'section', text: 'own' },
		parent_body: { from: 'section', text: 'enclosing_own' },
	},
};

function sectionConfiguration(base: string, kind: EvidenceKindDefinition): Configuration {
	return {
		id: 'html-sections',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/*.html'] },
		decision_model: { model: 'jev-latest' },
		evidence: { kind: 'inline', utilities: {}, matchers: {}, kinds: { section: kind } },
		rulebooks: [
			{
				kind: 'inline',
				id: 'section-rules',
				markdown: 'Each section keeps its heading markup. <!-- K001 -->\n',
				entries: {
					K001: {
						name: 'section_markup',
						evidence: 'section',
						severity: 'warn',
						decision: {
							start: 'raw',
							steps: {
								raw: {
									mechanical_check: { tool: 'contains', field: 'own_body', text: '<h' },
									yes: 'pass',
									no: { finding: 'Raw heading markup was lost.' },
								},
							},
						},
					},
				},
			},
		],
	};
}

const page =
	"<div><h2>Parent</h2>\n<p>One</p><img alt='Late diagram'>\n<section><h3>Child</h3><p>Child body</p></section><p>Tail</p></div><h2>Sibling</h2><p>Last</p>";

function fieldText(report: SdkReport, piece: number, field: string): unknown {
	const evidence = Object.values(report.piece_evidence)[piece];
	const value = evidence?.piece.fields[field];
	if (value?.state === 'text') return value.value;
	return value?.state;
}

await test('native HTML sections expose full, own and enclosing own text through the addon', async () => {
	const repository = ownedDirectory('html-sections');
	try {
		writeFiles(repository.path, { 'docs/page.html': page });
		const configuration = sectionConfiguration(repository.path, sectionKind);
		assert.equal((await check(configuration)).kind, 'valid');
		const outcome = await run(configuration, { root: repository.path, scope: { kind: 'all' } });
		assert.equal(outcome.kind, 'completed');
		if (outcome.kind !== 'completed') return;
		const report = outcome.report;
		// Raw mechanical text keeps the heading markup, so the rule passes.
		assert.equal(report.status.kind, 'clean');
		assert.equal(Object.keys(report.piece_evidence).length, 3);
		const parent = page.indexOf('<h2>Parent');
		const child = page.indexOf('<h3>');
		const sibling = page.indexOf('<h2>Sibling');
		assert.equal(Object.values(report.piece_evidence)[0]?.piece.text, page.slice(parent, sibling));
		assert.equal(fieldText(report, 0, 'own_body'), page.slice(parent, child));
		assert.equal(fieldText(report, 0, 'complete'), page.slice(parent, sibling));
		// A top-level section has no enclosing section.
		assert.equal(fieldText(report, 0, 'parent_body'), 'absent');
		assert.equal(fieldText(report, 1, 'parent_body'), page.slice(parent, child));
		assert.equal(fieldText(report, 1, 'own_body'), page.slice(child, sibling));
		assert.equal(fieldText(report, 2, 'complete'), page.slice(sibling));
		// The asking projection is text with attributes kept; the raw field stays markup.
		const view = Object.values(report.piece_evidence)[0]?.piece.model_view;
		assert.equal(view?.state, 'bounded');
		if (view?.state === 'bounded') {
			const own = view.fields['own_body'];
			assert.ok(JSON.stringify(own).includes('[alt: Late diagram]'), JSON.stringify(own));
			assert.ok(!JSON.stringify(own).includes('<p>'), 'the model sees text, not markup');
		}
		assert.equal(report.usage.model_requests, 0);
	} finally {
		repository.remove();
	}
});

/**
 * Check a value as a plain-JavaScript caller would, outside the declarations: the call
 * is made dynamically and its reply is narrowed before use.
 */
async function checkDynamically(value: unknown): Promise<CheckOutcome> {
	const reply: unknown = await Reflect.apply(check, undefined, [value]);
	if (!isCheckOutcome(reply)) throw new Error('check replied outside its closed outcome');
	return reply;
}

/** The decoded refusal of one selector: its code, path and reason. */
async function selectorRefusal(base: string, selection: unknown): Promise<unknown> {
	const configuration = sectionConfiguration(base, sectionKind);
	const outcome = await checkDynamically({
		...configuration,
		evidence: {
			...configuration.evidence,
			kinds: { section: { ...sectionKind, html_units: selection } },
		},
	});
	if (outcome.kind !== 'invalid') return outcome.kind;
	assert.equal(outcome.diagnostics.length, 1);
	const [diagnostic] = outcome.diagnostics;
	// serde_json appends the input position (" at line N column M"); it depends only on
	// the encoded configuration's length, so only that suffix is dropped.
	return [
		diagnostic?.code,
		diagnostic?.json_path,
		diagnostic?.message.replace(/ at line \d+ column \d+$/u, ''),
	];
}

await test('empty, invalid and doubly filtered section selectors are refused for their own reason', async () => {
	const base = ownedDirectory('html-section-refusals');
	try {
		// The native validator refuses an empty or unusable selector.
		const invalidSelector = [
			'syntax',
			['evidence'],
			'Evidence preparation refused: InvalidSelector',
		];
		// The closed wire refuses two filters or an unknown mode before validation.
		const noWireVariant = [
			'syntax',
			['evidence'],
			'data did not match any variant of untagged enum SelectionWire',
		];
		const cases: readonly (readonly [unknown, unknown])[] = [
			[{ mode: 'unit', units: [] }, invalidSelector],
			[{ mode: 'unit', units: ['section'], starts_with: '' }, invalidSelector],
			[{ mode: 'unit', units: ['section'], pattern: '(' }, invalidSelector],
			[{ mode: 'unit', units: ['section'], pattern: 'a*' }, invalidSelector],
			[{ mode: 'unit', units: ['section'], starts_with: 'A', pattern: 'B' }, noWireVariant],
			[{ mode: 'section', units: ['section'] }, noWireVariant],
			// Valid controls: unfiltered and each single filter.
			[{ mode: 'unit', units: ['section'] }, 'valid'],
			[{ mode: 'unit', units: ['section'], starts_with: 'Parent' }, 'valid'],
			[{ mode: 'unit', units: ['section'], pattern: '^Par' }, 'valid'],
		];
		const verdicts = await Promise.all(
			cases.map(async ([selection]) => selectorRefusal(base.path, selection)),
		);
		assert.deepEqual(
			verdicts,
			cases.map(([, expected]) => expected),
		);
	} finally {
		base.remove();
	}
});
