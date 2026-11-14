// Released section-prefix fields through the public inspection and run calls.
import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Configuration, InspectionReport } from '../src/index.ts';
import { check, inspect, run } from '../src/index.ts';
import { ownedDirectory, writeFiles } from './support/sdk-fixtures.ts';

type EvidencePiece = InspectionReport['pieces'][number];

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

function configuration(
	base: string,
	subjectPattern: string,
	containingKind = 'outline',
): Configuration {
	return {
		id: 'section-prefix',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/*.html'] },
		decision_model: { model: 'jev-latest' },
		evidence: {
			kind: 'inline',
			utilities: {},
			matchers: { subject: { language: 'Html', rule: { kind: 'element', regex: subjectPattern } } },
			kinds: {
				outline: { html_units: { mode: 'unit', units: ['section'] }, fields: {} },
				subject: {
					rule: 'subject',
					fields: { prefix: { from: 'section_prefix', kind: containingKind } },
				},
			},
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'prefix-rules',
				markdown: 'The prefix is complete. <!-- K001 -->\n',
				entries: {
					K001: {
						name: 'prefix_complete',
						evidence: 'subject',
						severity: 'error',
						decision: {
							start: 'complete',
							steps: {
								complete: {
									mechanical_check: { tool: 'complete', field: 'prefix' },
									yes: 'pass',
									no: { finding: 'Prefix incomplete.' },
								},
							},
						},
					},
				},
			},
		],
	};
}

/** Return only subject evidence, in source order, independent of report ordering. */
function subjects(pieces: readonly EvidencePiece[]): EvidencePiece[] {
	return pieces
		.filter((piece) => piece.kind === 'subject')
		.toSorted((left, right) => Number(left.where.span.byte_start - right.where.span.byte_start));
}

function expectPrefix(
	piece: EvidencePiece | undefined,
	page: string,
	start: number,
	end: number,
): void {
	assert.ok(piece !== undefined);
	const field = piece.fields['prefix'];
	assert.equal(field?.state, 'text');
	if (field?.state !== 'text') throw new Error('prefix is not text');
	assert.equal(field.value, page.slice(start, end));
	assert.equal(field.spans.length, 1);
	const span = field.spans[0];
	assert.ok(span !== undefined);
	assert.equal(span.byte_start, BigInt(Buffer.byteLength(page.slice(0, start))));
	assert.equal(span.byte_end, BigInt(Buffer.byteLength(page.slice(0, end))));
	assert.deepEqual(span.source, piece.where.span.source);
}

await test('inspection and run retain the nearest configured prefix and exact source spans', async () => {
	const repository = ownedDirectory('section-prefix');
	try {
		const page =
			'猫\n<h2>Outer</h2><p>Outer text</p><div><h3>Inner</h3><p>é brief</p><aside>FIRST</aside><p>Between</p><aside>SECOND</aside></div><h2>Peer</h2><p>LATER</p>';
		writeFiles(repository.path, { 'docs/page.html': page });
		const config = configuration(repository.path, '^<aside');
		assert.equal((await check(config)).kind, 'valid');
		const options = { root: repository.path, scope: { kind: 'all' as const } };
		const inspected = await inspect(config, options);
		assert.equal(inspected.kind, 'inspected');
		if (inspected.kind !== 'inspected') throw new Error('inspection did not complete');
		const observed = subjects(inspected.report.pieces);
		assert.equal(observed.length, 2);
		const start = page.indexOf('<h3>');
		expectPrefix(observed[0], page, start, page.indexOf('<aside>FIRST'));
		expectPrefix(observed[1], page, start, page.indexOf('<aside>SECOND'));
		const ran = await run(config, options);
		assert.equal(ran.kind, 'completed');
		if (ran.kind !== 'completed') throw new Error('run did not complete');
		assert.equal(ran.report.status.kind, 'clean');
		const executed = subjects(Object.values(ran.report.piece_evidence).map((item) => item.piece));
		assert.equal(executed.length, 2);
		expectPrefix(executed[0], page, start, page.indexOf('<aside>FIRST'));
		expectPrefix(executed[1], page, start, page.indexOf('<aside>SECOND'));
		assert.deepEqual(
			executed.map((piece) => piece.fields['prefix']),
			observed.map((piece) => piece.fields['prefix']),
		);
	} finally {
		repository.remove();
	}
});

await test('an empty prefix is a text span; a subject without a containing section is unavailable', async () => {
	const repository = ownedDirectory('section-prefix-boundaries');
	try {
		const options = { root: repository.path, scope: { kind: 'all' as const } };
		writeFiles(repository.path, { 'docs/page.html': '<h2>Start</h2>' });
		const emptyConfig = configuration(repository.path, '^<h2');
		const empty = await inspect(emptyConfig, options);
		assert.equal(empty.kind, 'inspected');
		if (empty.kind !== 'inspected') throw new Error('empty inspection did not complete');
		expectPrefix(subjects(empty.report.pieces)[0], '<h2>Start</h2>', 0, 0);
		const emptyRun = await run(emptyConfig, options);
		assert.equal(emptyRun.kind, 'completed');
		if (emptyRun.kind !== 'completed') throw new Error('empty run did not complete');
		assert.equal(emptyRun.report.status.kind, 'clean');
		writeFiles(repository.path, { 'docs/page.html': '<aside>Subject without a section</aside>' });
		const missingConfig = configuration(repository.path, '^<aside');
		const missing = await inspect(missingConfig, options);
		assert.equal(missing.kind, 'inspected');
		if (missing.kind !== 'inspected') throw new Error('missing inspection did not complete');
		assert.deepEqual(subjects(missing.report.pieces)[0]?.fields['prefix'], {
			state: 'unavailable',
			reason: 'unreadable',
		});
		const missingRun = await run(missingConfig, options);
		assert.equal(missingRun.kind, 'incomplete');
		if (missingRun.kind !== 'incomplete') throw new Error('missing section must end incomplete');
		assert.equal(missingRun.report.status.kind, 'incomplete');
	} finally {
		repository.remove();
	}
});

await test('undefined and cyclic section kind references fail ordinary admission', async () => {
	const repository = ownedDirectory('section-prefix-admission');
	try {
		const cases = [
			['missing', /undefined/u],
			['subject', /cycle/u],
		] as const;
		const outcomes = await Promise.all(
			cases.map(async ([kind]) => check(configuration(repository.path, '^<aside', kind))),
		);
		outcomes.forEach((outcome, index) => {
			assert.equal(outcome.kind, 'invalid');
			if (outcome.kind !== 'invalid') throw new Error('invalid reference admitted');
			const expected = cases[index]?.[1];
			assert.ok(expected !== undefined);
			assert.ok(outcome.diagnostics.some((diagnostic) => expected.test(diagnostic.message)));
		});
	} finally {
		repository.remove();
	}
});
