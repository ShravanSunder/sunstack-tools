import assert from 'node:assert/strict';

import { test } from 'vitest';

import { requireCaptureParityReceipt } from './capture-oracle.ts';
import { requireSameFrozenInput, type FrozenInput } from './frozen-inputs.ts';
import {
	fullOracleConfig,
	requireSamePreparedInputs,
	preparedSelectedFiles,
	preparedOwnerFiles,
} from './prepared-oracle.ts';

const config = {
	rulebook: [{ markdown: '../book.md', rules: 'rules.json' }],
	evidence: { ast_grep: 'evidence/sgconfig.yml', kinds: {} },
	check: { files: ['docs/*.md'] },
	decision_model: { model: 'declared-model' },
	limits: { max_matches: 77 },
};
test('external oracle changes only its declared cap and rebases actual config-relative inputs', () => {
	const original = structuredClone(config);
	const result = fullOracleConfig(config, '/frozen/plugin/rules/config.json', 10000);
	assert.deepEqual(result, {
		...config,
		rulebook: [{ markdown: '/frozen/plugin/book.md', rules: '/frozen/plugin/rules/rules.json' }],
		evidence: { ...config.evidence, ast_grep: '/frozen/plugin/rules/evidence/sgconfig.yml' },
		limits: { max_matches: 77, max_files: 10000 },
	});
	assert.deepEqual(config, original);
});
const location = {
	file: 'docs/right.md',
	first_line: 1,
	last_line: 1,
	span: {
		source: {
			kind: 'document',
			repository: 'checked_repository',
			path: 'docs/right.md',
			content_identity: 'a'.repeat(64),
		},
		byte_start: 0,
		byte_end: 5,
		start_line: 1,
		start_column: 1,
		end_line: 1,
		end_column: 6,
	},
};
const prepared = {
	version: '0.11.1',
	mode: 'prepare',
	status: { kind: 'prepared' },
	usage: { model_requests: 0 },
	file_counts: [
		{
			file: 'docs/right.md',
			kinds: { paragraph: { matches: 1, pieces: 1, picked: 1, limited: false } },
		},
	],
	results: [{ file: 'docs/right.md', id: 1 }],
	requests: [
		{
			id: 1,
			owner: { kind: 'rule', rule: 'body-present', piece: location },
			request: { questions: { questionOne: 'original' }, state: { body: 'original' } },
		},
	],
	filter_answers: [],
	filter_skips: [],
	selections: [],
	needs_answer: [],
};
const candidate = { ...prepared, version: '0.12.0' };
const compare = (actual: typeof candidate): void =>
	requireSamePreparedInputs({
		candidate: actual,
		full: prepared,
		candidateVersion: '0.12.0',
		fullVersion: '0.11.1',
	});
test('accepts exact prepared inputs across explicitly declared root versions', () => {
	compare(candidate);
	assert.deepEqual(preparedSelectedFiles(candidate), ['docs/right.md']);
});
test('rejects another file owner despite equal result/request counts', () => {
	assert.throws(() => compare({ ...candidate, results: [{ file: 'docs/wrong.md', id: 1 }] }));
});
test('rejects changed raw question/state payload with the same owner and count', () => {
	const changed = structuredClone(candidate);
	const firstRequest = changed.requests[0];
	assert.ok(firstRequest);
	firstRequest.request.state.body = 'wrong';
	assert.throws(() => compare(changed));
});
test('rejects a missing request instead of treating preparation success as parity', () => {
	assert.throws(() => compare({ ...candidate, requests: [] }));
});
test('rejects the wrong declared version', () => {
	assert.throws(() => compare({ ...candidate, version: '0.11.1' }));
});
test('rejects a non-prepared result and any provider dispatch', () => {
	assert.throws(() => compare({ ...candidate, status: { kind: 'incomplete' } }));
	assert.throws(() => compare({ ...candidate, usage: { model_requests: 1 } }));
});
const input: FrozenInput = {
	root: '/frozen',
	head: 'declared',
	base: 'base',
	trackedCount: 2,
	files: { 'one.md': 'file:420:digest', 'CLAUDE.md': 'link:511:"AGENTS.md"' },
	gitAdmin: { HEAD: 'head', index: 'index' },
};
test('frozen identity guard catches a link payload change even when regular files stay equal', () => {
	assert.throws(() =>
		requireSameFrozenInput(input, {
			...input,
			files: { ...input.files, 'CLAUDE.md': 'link:511:"elsewhere"' },
		}),
	);
});
test('frozen identity guard rejects source mode and Git-index changes', () => {
	assert.throws(() =>
		requireSameFrozenInput(input, {
			...input,
			files: { ...input.files, 'one.md': 'file:493:digest' },
		}),
	);
	assert.throws(() =>
		requireSameFrozenInput(input, { ...input, gitAdmin: { ...input.gitAdmin, index: 'changed' } }),
	);
});

const nativeInput: FrozenInput = {
	root: '/frozen',
	head: 'head',
	base: 'base',
	trackedCount: 1,
	files: { 'one.md': `file:420:${'a'.repeat(64)}` },
	gitAdmin: { index: 'index' },
};
const nativeReceipt = {
	version: '0.12.0',
	root: '/frozen',
	base_revision: 'base',
	git_guard: { head: 'head', index_sha256: 'index' },
	input_fingerprints: { 'one.md': `file:${'a'.repeat(64)}` },
	limits: { max_files: 2000 },
	oracle_max_files: 10000,
	changed_only: { acquired_files: 2, paths: ['one.md'] },
	full: { acquired_files: 4, paths: ['one.md'] },
	raw_changes: [
		[
			{
				path: 'one.md',
				before_sha256: 'b'.repeat(64),
				after_sha256: 'a'.repeat(64),
				before_spans: [],
				after_spans: [],
			},
		],
	],
	raw_changes_equal: true,
	input_unchanged: true,
};
test('native receipt binds the same immutable input and a real acquisition reduction', () => {
	assert.deepEqual(requireCaptureParityReceipt(nativeReceipt, nativeInput), {
		changedFiles: 2,
		fullFiles: 4,
		paths: ['one.md'],
	});
});
test('native receipt cannot be reused from another root or Git index', () => {
	assert.throws(() =>
		requireCaptureParityReceipt({ ...nativeReceipt, root: '/different' }, nativeInput),
	);
	assert.throws(() =>
		requireCaptureParityReceipt(
			{ ...nativeReceipt, git_guard: { head: 'head', index_sha256: 'other' } },
			nativeInput,
		),
	);
});
test('native receipt rejects an empty changed-set witness and changed/full disagreement', () => {
	assert.throws(() =>
		requireCaptureParityReceipt(
			{
				...nativeReceipt,
				changed_only: { acquired_files: 0, paths: [] },
				full: { acquired_files: 4, paths: [] },
				raw_changes: [],
			},
			nativeInput,
		),
	);
	assert.throws(() =>
		requireCaptureParityReceipt(
			{ ...nativeReceipt, full: { acquired_files: 4, paths: ['other.md'] } },
			nativeInput,
		),
	);
});
test('native receipt rejects equal counts or acquisitions unrelated to its changed paths', () => {
	assert.throws(() =>
		requireCaptureParityReceipt(
			{ ...nativeReceipt, changed_only: { acquired_files: 4, paths: ['one.md'] } },
			nativeInput,
		),
	);
	assert.throws(() =>
		requireCaptureParityReceipt(
			{ ...nativeReceipt, changed_only: { acquired_files: 3, paths: ['one.md'] } },
			nativeInput,
		),
	);
});
test('native receipt rejects missing file identity and raw rows for the wrong path', () => {
	assert.throws(() =>
		requireCaptureParityReceipt({ ...nativeReceipt, input_fingerprints: {} }, nativeInput),
	);
	assert.throws(() =>
		requireCaptureParityReceipt(
			{
				...nativeReceipt,
				raw_changes: [
					[
						{
							path: 'other.md',
							before_sha256: 'b'.repeat(64),
							after_sha256: 'a'.repeat(64),
							before_spans: [],
							after_spans: [],
						},
					],
				],
			},
			nativeInput,
		),
	);
});

test('picked file accounting includes selected pieces with no result or request', () => {
	const report = {
		results: [],
		requests: [],
		selections: [],
		file_counts: [
			{
				file: 'docs/picked.md',
				kinds: { paragraph: { matches: 1, pieces: 1, picked: 1, limited: false } },
			},
			{
				file: 'docs/unpicked.md',
				kinds: { paragraph: { matches: 1, pieces: 1, picked: 0, limited: false } },
			},
		],
	};
	assert.deepEqual(preparedSelectedFiles(report), ['docs/picked.md']);
});

test('actual saved owner variants and selection where locations stay distinct from picked files', () => {
	const report = {
		results: [],
		requests: [
			{ owner: { kind: 'rule', rule: 'body-present', piece: location } },
			{ owner: { kind: 'selection', selection_kind: 'paragraph', file: 'docs/candidate.md' } },
			{ owner: { kind: 'filter', filter: 'scoped', folder: 'docs' } },
		],
		selections: [
			{
				kind: 'paragraph',
				where: { ...location, file: 'docs/location.md' },
				first_line: 'BODY',
				outcome: { kind: 'prepared', request_id: 2 },
			},
		],
		file_counts: [
			{
				file: 'docs/picked.md',
				kinds: { paragraph: { matches: 1, pieces: 1, picked: 1, limited: false } },
			},
		],
	};
	assert.deepEqual(preparedSelectedFiles(report), ['docs/picked.md']);
	assert.deepEqual(preparedOwnerFiles(report), [
		'docs/candidate.md',
		'docs/location.md',
		'docs/right.md',
	]);
});
test('invented top-level rule file cannot silently stand in for Rule.piece.file', () => {
	assert.throws(() =>
		preparedOwnerFiles({
			results: [],
			requests: [{ owner: { kind: 'rule', rule: 'body-present', file: 'docs/right.md' } }],
			selections: [],
		}),
	);
});
