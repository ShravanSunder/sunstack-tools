// Declared contract shapes against what the real addon accepts and returns: structured
// numbers, upstream matcher defaults, present-only fields, the selection question and
// full-width range positions.
import assert from 'node:assert/strict';
import { test } from 'node:test';

import type {
	AstMatcher,
	CheckOutcome,
	Configuration,
	EvidenceKindDefinition,
} from '../src/index.ts';
import { check, load } from '../src/index.ts';
import { isCheckOutcome } from '../src/reply-decoding.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

const paragraphKind: EvidenceKindDefinition = {
	rule: 'paragraph',
	fields: { body: { from: 'remainder' } },
};

function configurationWith(
	base: string,
	parts: { matcher?: AstMatcher; kind?: EvidenceKindDefinition; reference?: unknown },
): Configuration {
	return {
		id: 'contract-values',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/*.md'] },
		decision_model: { model: 'jev-latest' },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: {
				paragraph: parts.matcher ?? { language: 'Markdown', rule: { kind: 'paragraph' } },
			},
			kinds: { paragraph: parts.kind ?? paragraphKind },
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'contract-rules',
				markdown: 'Each paragraph has a body. <!-- K001 -->\n',
				entries: {
					K001: {
						name: 'body',
						evidence: 'paragraph',
						severity: 'error',
						decision: {
							start: 'has',
							steps: {
								has: {
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

/** Check a value as a plain-JavaScript caller would, outside the declarations. */
async function checkDynamically(value: unknown): Promise<CheckOutcome> {
	const reply: unknown = await Reflect.apply(check, undefined, [value]);
	if (!isCheckOutcome(reply)) throw new Error('check replied outside its closed outcome');
	return reply;
}

/** A configuration with one inline value replaced, as a dynamic caller could send it. */
function replaced(configuration: Configuration, path: readonly string[], value: unknown): unknown {
	const copy: unknown = structuredClone(configuration);
	let target: unknown = copy;
	for (const key of path.slice(0, -1)) {
		assert.ok(typeof target === 'object' && target !== null && key in target);
		target = Reflect.get(target, key);
	}
	assert.ok(typeof target === 'object' && target !== null);
	Reflect.set(target, path.at(-1) ?? '', value);
	return copy;
}

await test('structured references load exact integers as bigint and floats as number', async () => {
	const project = ownedDirectory('structured-reference');
	try {
		writeFiles(project.path, {
			'sunstack.lint.json': JSON.stringify({
				rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
				marker_prefix: 'K',
				check: { files: ['docs/*.md'] },
				evidence: {
					ast_grep: 'evidence/sgconfig.yml',
					kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
				},
				decision_model: { model: 'jev-latest' },
			}),
			'rulebook.md': 'Each paragraph has a body. <!-- K001 -->\n',
			'rules.json':
				'{"K001":{"name":"body","evidence":"paragraph","severity":"error","reference":{"threshold":2,"ratio":1.5,"wide":18446744073709551615,"adjacent":18446744073709551614,"lowest":-9223372036854775808,"nested":{"list":[3,0.25]}},"decision":{"start":"has","steps":{"has":{"mechanical_check":{"tool":"complete","field":"body"},"yes":"pass","no":{"finding":"Empty."}}}}}}',
			'evidence/sgconfig.yml': 'ruleDirs: [rules]\n',
			'evidence/rules/paragraph.yml':
				'id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n',
		});
		const loaded = await load({ directory: project.path });
		assert.equal(loaded.kind, 'loaded');
		if (loaded.kind !== 'loaded') return;
		const [rulebook] = loaded.value.configuration.rulebooks;
		assert.equal(rulebook?.kind, 'captured');
		if (rulebook?.kind !== 'captured') return;
		const reference = rulebook.entries.value['K001']?.reference;
		assert.deepEqual(reference, {
			threshold: 2n,
			ratio: 1.5,
			wide: 18446744073709551615n,
			adjacent: 18446744073709551614n,
			lowest: -9223372036854775808n,
			nested: { list: [3n, 0.25] },
		});
		// The exact values round-trip: unchanged keeps file provenance, an adjacent edit does not.
		const unchanged = await check(loaded.value);
		assert.ok(
			unchanged.kind === 'valid' &&
				unchanged.sources.every((source) => source.origin.kind === 'file'),
		);
		const entry = rulebook.entries.value['K001'];
		assert.ok(entry !== undefined && reference !== undefined);
		const edited = await check({
			...loaded.value.configuration,
			rulebooks: [
				{
					...rulebook,
					entries: {
						...rulebook.entries,
						value: { K001: { ...entry, reference: { ...reference, wide: 18446744073709551614n } } },
					},
				},
			],
		});
		assert.equal(edited.kind, 'valid');
		if (edited.kind === 'valid') {
			const entries = edited.sources.find((source) => source.origin.kind === 'edited');
			assert.ok(entries?.content.includes('"wide":18446744073709551614'));
		}
	} finally {
		project.remove();
	}
});

await test('upstream matcher defaults: fix may be null, message and severity may not', async () => {
	const base = ownedDirectory('matcher-defaults');
	try {
		const withFixNull = configurationWith(base.path, {
			matcher: { language: 'Markdown', rule: { kind: 'paragraph' }, fix: null },
		});
		assert.equal((await check(withFixNull)).kind, 'valid');
		const fields = ['message', 'severity'] as const;
		const refusals = await Promise.all(
			fields.map(async (field) =>
				checkDynamically(replaced(withFixNull, ['evidence', 'matchers', 'paragraph', field], null)),
			),
		);
		for (const [index, refused] of refusals.entries()) {
			assert.equal(
				refused.kind,
				'invalid',
				`${fields[index]}: null is refused by the upstream decoder`,
			);
		}
		// A loaded file matcher returns the upstream defaults: fix null, message '', severity hint.
		writeFiles(base.path, {
			'project/sunstack.lint.json': JSON.stringify({
				rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
				marker_prefix: 'K',
				check: { files: ['docs/*.md'] },
				evidence: {
					ast_grep: 'evidence/sgconfig.yml',
					kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
				},
				decision_model: { model: 'jev-latest' },
			}),
			'project/rulebook.md': 'Each paragraph has a body. <!-- K001 -->\n',
			'project/rules.json':
				'{"K001":{"name":"body","evidence":"paragraph","severity":"error","decision":{"start":"has","steps":{"has":{"mechanical_check":{"tool":"complete","field":"body"},"yes":"pass","no":{"finding":"Empty."}}}}}}',
			'project/evidence/sgconfig.yml': 'ruleDirs: [rules]\n',
			'project/evidence/rules/paragraph.yml':
				'id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n',
		});
		const loaded = await load({ directory: `${base.path}/project` });
		assert.equal(loaded.kind, 'loaded');
		if (loaded.kind === 'loaded' && loaded.value.configuration.evidence.kind === 'captured') {
			const matcher = loaded.value.configuration.evidence.matchers['paragraph']?.value;
			assert.equal(matcher?.fix, null);
			assert.equal(matcher?.message, '');
			assert.equal(matcher?.severity, 'hint');
		}
	} finally {
		base.remove();
	}
});

await test('present-only fields are refused as null and accepted when omitted', async () => {
	const base = ownedDirectory('present-only');
	try {
		const valid = configurationWith(base.path, {});
		assert.equal((await check(valid)).kind, 'valid');
		const nullFields: readonly (readonly string[])[] = [
			['evidence', 'kinds', 'paragraph', 'within'],
			['evidence', 'kinds', 'paragraph', 'inline_rule'],
			['evidence', 'kinds', 'paragraph', 'html_units'],
			['rulebooks', '0', 'entries', 'K001', 'only_when'],
		];
		const outcomes = await Promise.all(
			nullFields.map(async (path) => checkDynamically(replaced(valid, path, null))),
		);
		for (const [index, outcome] of outcomes.entries()) {
			assert.equal(outcome.kind, 'invalid', `${nullFields[index]?.join('.')}: null is refused`);
		}
		// A noul question's criteria may be omitted, never null.
		const asking = replaced(valid, ['rulebooks', '0', 'entries', 'K001', 'questions'], {
			asked: { type: 'noul', instructions: 'Is it clear?', criteria: null },
		});
		assert.equal((await checkDynamically(asking)).kind, 'invalid');
	} finally {
		base.remove();
	}
});

await test('a selection question must be a noul judgment question', async () => {
	const base = ownedDirectory('selection-question');
	try {
		const selected: EvidenceKindDefinition = {
			...paragraphKind,
			select: {
				question: { type: 'noul', instructions: { check: 'body', task: 'Is this a procedure?' } },
				yes_at_or_above: 0.8,
				no_at_or_below: 0.2,
			},
		};
		const valid = configurationWith(base.path, { kind: selected });
		assert.equal((await check(valid)).kind, 'valid');
		const refusals = await Promise.all(
			[
				false,
				{ type: 'choice', instructions: 'Which?', criteria: { a: 'A' } },
				{ type: 'noul', instruction: 'Is it?' },
			].map(async (question) =>
				checkDynamically(
					replaced(valid, ['evidence', 'kinds', 'paragraph', 'select', 'question'], question),
				),
			),
		);
		for (const outcome of refusals) assert.equal(outcome.kind, 'invalid');
	} finally {
		base.remove();
	}
});

await test('full-width range positions cross exactly; unsafe numbers are refused before dispatch', async () => {
	const base = ownedDirectory('range-positions');
	try {
		const line = 9007199254740993n;
		const wide = configurationWith(base.path, {
			matcher: {
				language: 'Markdown',
				rule: {
					kind: 'paragraph',
					range: { start: { line, column: 0 }, end: { line, column: 1n } },
				},
			},
		});
		const admitted = await check(wide);
		assert.equal(admitted.kind, 'valid');
		if (admitted.kind === 'valid') {
			const matcher = admitted.sources.find((source) => source.content.includes('"range"'));
			assert.ok(
				matcher?.content.includes('"line":9007199254740993'),
				matcher?.content ?? 'no range source',
			);
		}
		const small = configurationWith(base.path, {
			matcher: {
				language: 'Markdown',
				rule: {
					kind: 'paragraph',
					range: { start: { line: 1, column: 0 }, end: { line: 2, column: 4 } },
				},
			},
		});
		assert.equal((await check(small)).kind, 'valid');
		const unsafe = await checkDynamically(
			replaced(
				small,
				['evidence', 'matchers', 'paragraph', 'rule', 'range', 'start', 'line'],
				// An unsafe integer number cannot be sent exactly; bigint is required.
				Number.MAX_SAFE_INTEGER + 2,
			),
		);
		assert.deepEqual(
			unsafe.kind === 'failed' ? [unsafe.failure.phase, unsafe.failure.reason] : unsafe.kind,
			['before_dispatch', 'encoding_rejected'],
		);
		// A file matcher's wide position loads as exact bigint.
		writeFiles(base.path, {
			'project/sunstack.lint.json': JSON.stringify({
				rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
				marker_prefix: 'K',
				check: { files: ['docs/*.md'] },
				evidence: {
					ast_grep: 'evidence/sgconfig.yml',
					kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
				},
				decision_model: { model: 'jev-latest' },
			}),
			'project/rulebook.md': 'Each paragraph has a body. <!-- K001 -->\n',
			'project/rules.json':
				'{"K001":{"name":"body","evidence":"paragraph","severity":"error","decision":{"start":"has","steps":{"has":{"mechanical_check":{"tool":"complete","field":"body"},"yes":"pass","no":{"finding":"Empty."}}}}}}',
			'project/evidence/sgconfig.yml': 'ruleDirs: [rules]\n',
			'project/evidence/rules/paragraph.yml':
				'id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph, range: {start: {line: 9007199254740993, column: 0}, end: {line: 9007199254740993, column: 1}}}\n',
		});
		const loaded = await load({ directory: `${base.path}/project` });
		assert.equal(loaded.kind, 'loaded');
		if (loaded.kind === 'loaded' && loaded.value.configuration.evidence.kind === 'captured') {
			const rule = loaded.value.configuration.evidence.matchers['paragraph']?.value.rule;
			assert.deepEqual(rule?.range?.start, { line: 9007199254740993n, column: 0 });
		}
	} finally {
		base.remove();
	}
});
