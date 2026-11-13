// Evidence selection requests through the addon against a controlled backend: candidate and
// shared views cut by the configured budget name their cut fields in `cut-fields`; uncut
// views omit it, and authored "[cut]" text never creates it. The report's saved request and
// its input hash carry the same metadata as the HTTP request the model received.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { test } from 'node:test';

import type { Configuration, SdkReport } from '../src/index.ts';
import { run } from '../src/index.ts';
import type { CapturedRequest } from './support/controlled-backend.ts';
import { withAnsweringBackend } from './support/controlled-backend.ts';
import { writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

/** The authored text of the short section: cut-marker look-alikes that must stay literal. */
const literalMarkers = 'Authored [cut] and [cut: 2 more] stay literal.';
/** The short section's own text: its heading line and the literal markers. */
const shortSection = `# Short\n\n${literalMarkers}\n`;

const pages = {
	// A long section, cut by the evidence budget, beside a short one that is not.
	'docs/long.md': `# Long\n\n${'body text '.repeat(300).trim()}\n\n${shortSection}`,
	// Nothing in this file reaches the budget.
	'docs/small.md': '# Small\n\nTiny.\n',
} as const;

function selectionConfiguration(base: string, endpoint: string): Configuration {
	return {
		id: 'selection-cuts',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/*.md'] },
		decision_model: { model: 'jev-latest', endpoint },
		limits: { evidence_max_chars: 1000 },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: {},
			kinds: {
				// The shared context reads every section of the file through a separate kind.
				outline: { markdown_units: { mode: 'unit', units: ['section'] }, fields: {} },
				section: {
					markdown_units: { mode: 'unit', units: ['section'] },
					fields: {
						body: { from: 'section', text: 'own' },
						outline: { from: 'same_file', kind: 'outline', text: 'full' },
					},
					select: {
						question: {
							type: 'noul',
							instructions: { check: 'body', context: ['outline'], task: 'Keep this section?' },
						},
						yes_at_or_above: 0.7,
						no_at_or_below: 0.3,
						shared: ['outline'],
					},
				},
			},
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'selection-rules',
				markdown: 'Every selected section has a body. <!-- K001 -->\n',
				entries: {
					K001: {
						name: 'has_body',
						evidence: 'section',
						severity: 'warn',
						decision: {
							start: 'complete',
							steps: {
								complete: {
									mechanical_check: { tool: 'complete', field: 'body' },
									yes: 'pass',
									no: { finding: 'Empty.' },
								},
							},
						},
					},
				},
			},
		],
	};
}

function record(value: unknown): Readonly<Record<string, unknown>> {
	assert.ok(typeof value === 'object' && value !== null && !Array.isArray(value));
	return Object.fromEntries(Object.entries(value));
}

/** The `where.file` a selection request's state names. */
function requestFile(state: Readonly<Record<string, unknown>>): string {
	const file = record(state['where'])['file'];
	assert.equal(typeof file, 'string');
	return String(file);
}

/**
 * SHA-256 of the compact JSON of a saved request, as the engine digests it: the
 * StructuredJudgmentRequest fields in declaration order, `questions` then `state`.
 */
function requestDigest(state: unknown, questions: unknown): string {
	return createHash('sha256').update(JSON.stringify({ questions, state })).digest('hex');
}

const byText = (left: string, right: string): number => left.localeCompare(right);

interface SelectionRun {
	readonly report: SdkReport;
	readonly requests: readonly CapturedRequest[];
}

await test('selection requests name cut candidate and shared fields, omit uncut ones and keep authored markers literal', async (context) => {
	const { value, answered } = await withAnsweringBackend(
		'selection-cuts',
		8,
		async (backend, root): Promise<SelectionRun> => {
			writeFiles(root, pages);
			const outcome = await run(selectionConfiguration(root, backend.endpoint), {
				root,
				scope: { kind: 'all' },
			});
			assert.equal(
				outcome.kind,
				'completed',
				outcome.kind === 'invalid'
					? outcome.diagnostics.map((item) => item.message).join('; ')
					: '',
			);
			if (outcome.kind !== 'completed') throw new Error('unreachable');
			return { report: outcome.report, requests: [...backend.received] };
		},
	);
	const { report, requests } = value;
	// One selection request per file; no rule asks the model.
	assert.equal(answered, 2);
	assert.equal(report.status.kind, 'clean');
	const sent = new Map(
		requests.map((request) => {
			const state = record(request.body['state']);
			return [requestFile(state), request.body] as const;
		}),
	);
	const saved = new Map(
		report.requests.map(
			(request) => [requestFile(record(request.request.state)), request] as const,
		),
	);
	assert.deepEqual([...sent.keys()].toSorted(byText), ['docs/long.md', 'docs/small.md']);
	assert.deepEqual([...saved.keys()].toSorted(byText), ['docs/long.md', 'docs/small.md']);

	// The cut file: the long candidate and the shared outline name their cut fields; the short
	// candidate, whose own text only looks like cut markers, has none.
	const long = sent.get('docs/long.md');
	assert.ok(long !== undefined);
	const longState = record(long['state']);
	const candidates = record(longState['candidates']);
	const cutCandidate = record(candidates['c1']);
	const literalCandidate = record(candidates['c2']);
	const shared = record(longState['shared']);
	assert.deepEqual(cutCandidate['cut-fields'], ['body']);
	assert.deepEqual(shared['cut-fields'], ['outline']);
	assert.equal('cut-fields' in literalCandidate, false);
	assert.equal(literalCandidate['body'], shortSection);
	// The metadata comes from the real projection: it equals each bounded view's own cut list,
	// and the projected cut text is unchanged beside it.
	const longSaved = saved.get('docs/long.md');
	const views = longSaved?.selection_evidence;
	assert.ok(views !== undefined && views !== null);
	const [cutView, literalView] = views.candidates.map((candidate) => candidate.selection_view);
	assert.ok(cutView?.state === 'bounded' && literalView?.state === 'bounded');
	assert.ok(views.shared.view.state === 'bounded');
	assert.deepEqual(cutView.cut, ['body']);
	assert.deepEqual(literalView.cut, []);
	assert.deepEqual(views.shared.view.cut, ['outline']);
	assert.equal(cutCandidate['body'], cutView.fields['body']);
	const cutBody = cutCandidate['body'];
	assert.ok(typeof cutBody === 'string' && cutBody.includes('[cut'));

	// The uncut file: no view in the request carries the key.
	const small = sent.get('docs/small.md');
	assert.ok(small !== undefined);
	assert.doesNotMatch(JSON.stringify(small), /cut-fields/u);

	// The saved request is the request the model received, and its input hash covers the
	// metadata: removing it would give another digest.
	for (const [file, body] of sent) {
		const savedRequest = saved.get(file);
		assert.ok(savedRequest !== undefined);
		// Receipt: the full HTTP envelope and the returned request facts, in the test output.
		context.diagnostic(
			`selection request ${file}: ${JSON.stringify({
				http_body: body,
				saved_input_sha256: savedRequest.input_sha256,
				saved_state: savedRequest.request.state,
			})}`,
		);
		assert.deepEqual(savedRequest.request.state, body['state'], `${file} state`);
		assert.equal(
			savedRequest.input_sha256,
			requestDigest(body['state'], body['questions']),
			`${file} digest`,
		);
	}
	const withoutMetadata: unknown = JSON.parse(JSON.stringify(longState), (key, item: unknown) =>
		key === 'cut-fields' ? undefined : item,
	);
	assert.notEqual(
		saved.get('docs/long.md')?.input_sha256,
		requestDigest(withoutMetadata, long['questions']),
	);
});
