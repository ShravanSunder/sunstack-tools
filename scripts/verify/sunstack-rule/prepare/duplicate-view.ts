/** Duplicate source ranges must remain evidence and must never justify a guessed model view. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	baselineGit,
	configureEndpoint,
	objectValue,
	parseObject,
	writeJson,
	numberValue,
	textValue,
} from '../rulebook/rulebook-fixture-support.ts';
import { type RulebookProcess, responseBody } from '../rulebook/rulebook-process.ts';
import { preparedInventory, requireRequestParity } from './prepare-checks.ts';
import { prepareConfig, prepareRules } from './prepare-fixtures.ts';
import { judgedFixtureRule } from './review-fixtures.ts';

export function duplicateRangeFixture(root: string): void {
	filesystem.mkdirSync(path.join(root, 'evidence/rules'), { recursive: true });
	filesystem.mkdirSync(path.join(root, 'docs'));
	filesystem.writeFileSync(path.join(root, 'evidence/sgconfig.yml'), 'ruleDirs: [rules]\n');
	const prefix = 'DUPLICATE_RANGE_SENTINEL_';
	filesystem.writeFileSync(
		path.join(root, 'docs/one.md'),
		prefix + 'x'.repeat(800 - prefix.length),
	);
	writeJson(path.join(root, prepareRules), {
		K001: judgedFixtureRule('duplicate_plain', 'review', 'plain'),
	});
	filesystem.writeFileSync(
		path.join(root, 'rulebook.md'),
		'Read the plain document evidence. <!-- K001 -->\n',
	);
	writeJson(path.join(root, prepareConfig), {
		rulebook: [{ markdown: 'rulebook.md', rules: prepareRules }],
		marker_prefix: 'K',
		check: { files: ['docs/one.md'] },
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: {
				selected_context: {
					inline_rule: '{language: Markdown, rule: {kind: document}}',
					fields: { body: { from: 'first_line' } },
					select: {
						question: {
							type: 'noul',
							instructions: {
								check: 'body',
								task: 'Does this controlled document supply the selected context?',
							},
						},
						yes_at_or_above: 0.8,
						no_at_or_below: 0.2,
					},
				},
				known_context: {
					inline_rule: '{language: Markdown, rule: {any: [{kind: document}, {kind: paragraph}]}}',
					within: ['selected_context'],
					fields: { body: { from: 'first_line' } },
				},
				review: {
					inline_rule: '{language: Markdown, rule: {kind: document}}',
					max_chars: 2000,
					fields: {
						pending: { from: 'same_file', kind: 'known_context', text: 'full' },
						plain: { from: 'first_line' },
					},
				},
			},
		},
		decision_model: { model: 'typesafe/controlled-rulebook' },
		limits: {
			request_max_chars: 50000,
			max_input_tokens_per_request: 10000,
			max_model_requests: 20,
			max_concurrent_requests: 1,
		},
	});
	baselineGit(root);
}
export async function observeDuplicateRange(options: {
	observer: RulebookProcess;
	evidence: string;
	prepareExit: number;
}): Promise<void> {
	const root = path.join(options.evidence, 'duplicate-range-fixture');
	duplicateRangeFixture(root);
	configureEndpoint(root, options.observer.endpoint);
	options.observer.respond = ({ body }) =>
		Promise.resolve({ status: 200, body: responseBody(body, 0.95) });
	const args = ['lint', '--config', prepareConfig, '--all'];
	await options.observer.run({
		root,
		name: 'duplicate-range-check',
		arguments: ['lint', '--config', prepareConfig, '--check'],
		expectedExit: 0,
	});
	const ordinaryFile = path.join(options.evidence, 'duplicate-range.ordinary.json');
	const ordinary = await options.observer.run({
		root,
		name: 'duplicate-range-ordinary',
		arguments: [...args, '--report', ordinaryFile, '--format', 'json'],
		credential: true,
		expectedExit: 0,
	});
	assert.equal(
		ordinary.captures.length,
		2,
		'Duplicate witness did not select context and then judge its document',
	);
	const ordinaryRaw = filesystem.readFileSync(ordinaryFile, 'utf8');
	const report = parseObject(ordinaryRaw);
	const records = arrayValue(report['requests']).map(objectValue);
	const plain = records.find(
		(record) => objectValue(record['owner'])['rule'] === 'duplicate_plain',
	);
	assert.ok(plain);
	const piece = objectValue(
		objectValue(objectValue(report['piece_evidence'])[String(numberValue(plain['id']))])['piece'],
	);
	const fields = objectValue(piece['fields']);
	const values = arrayValue(objectValue(fields['pending'])['values']);
	assert.equal(values.length, 2, 'Document/paragraph duplicate ranges were not both retained');
	assert.equal(values[0], values[1]);
	assert.equal(typeof values[0], 'string');
	assert.equal(textValue(values[0]).length, 800);
	const cut = arrayValue(objectValue(objectValue(plain['request'])['state'])['cut-fields']);
	assert.ok(cut.includes('pending'), 'Ordinary duplicate evidence did not activate shared cuts');
	const waitingFile = path.join(options.evidence, 'duplicate-range.waiting.json');
	const waiting = await options.observer.run({
		root,
		name: 'duplicate-range-waiting',
		arguments: [...args, '--prepare', waitingFile],
		expectedExit: options.prepareExit,
	});
	assert.equal(waiting.captures.length, 0);
	const waitingRaw = filesystem.readFileSync(waitingFile, 'utf8');
	const pending = parseObject(waitingRaw);
	assert.equal(
		preparedInventory(waitingRaw).size,
		1,
		'Duplicate-range estimate fabricated an extra prepared request',
	);
	const selectors = arrayValue(pending['requests']).map(objectValue);
	assert.equal(objectValue(selectors[0]?.['owner'])['kind'], 'selection');
	const waits = arrayValue(pending['needs_answer']).map(objectValue);
	assert.ok(
		waits.some(
			(wait) =>
				wait['kind'] === 'review' &&
				wait['file'] === 'docs/one.md' &&
				objectValue(wait['waits_on'])['request_id'] === selectors[0]?.['id'],
		),
		'Duplicate composed dependency lost selection provenance',
	);
	const replayFile = path.join(options.evidence, 'duplicate-range.replayed.json');
	const replay = await options.observer.run({
		root,
		name: 'duplicate-range-replayed',
		arguments: [...args, '--prepare', replayFile, '--selections-from', ordinaryFile],
		expectedExit: options.prepareExit,
	});
	assert.equal(replay.captures.length, 0);
	const replayRaw = filesystem.readFileSync(replayFile, 'utf8');
	assert.deepEqual(parseObject(replayRaw)['needs_answer'], []);
	requireRequestParity({
		prepared: replayRaw,
		ordinary: ordinaryRaw,
		replayedFrom: ordinaryRaw,
		captures: ordinary.captures.map((capture) => capture.raw.toString('utf8')),
		expectedPrepared: 2,
	});
	writeJson(path.join(options.evidence, 'duplicate-range-observed.json'), {
		duplicate_values: 2,
		chars_each: 800,
		view_budget: 2000,
		unresolved_requests: 1,
		replay_requests: 2,
		no_prepare_http: true,
	});
}
