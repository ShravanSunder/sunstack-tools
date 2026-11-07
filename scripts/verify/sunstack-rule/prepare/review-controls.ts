/** Real CLI counterexamples from review F1/F2; expected payloads come only from actual dispatch. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	configureEndpoint,
	objectValue,
	parseObject,
	readObject,
	runSequence,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { type RulebookProcess, responseBody } from '../rulebook/rulebook-process.ts';
import { researchFile } from '../selection/selection-fixture.ts';
import {
	preparedInventory,
	requireRequestParity,
	requireSelectionWaits,
} from './prepare-checks.ts';
import { prepareConfig } from './prepare-fixtures.ts';
import { requirePayloadBytes } from './raw-json.ts';
import { composedSelectionFixture, failedAdmissionFixture } from './review-fixtures.ts';

function requests(raw: string): JsonObject[] {
	return arrayValue(parseObject(raw)['requests']).map(objectValue);
}
function byRule(records: readonly JsonObject[], rule: string): JsonObject[] {
	return records.filter(
		(record) =>
			objectValue(record['owner'])['kind'] === 'rule' &&
			objectValue(record['owner'])['rule'] === rule,
	);
}
async function observeProjectionDependency(options: {
	observer: RulebookProcess;
	root: string;
	evidence: string;
	prepareExit: number;
}): Promise<void> {
	const configFile = path.join(options.root, prepareConfig);
	const config = readObject(configFile);
	const kind = objectValue(objectValue(objectValue(config['evidence'])['kinds'])['z_composed']);
	kind['max_chars'] = 1000;
	writeJson(configFile, config);
	const args = ['lint', '--config', prepareConfig, '--all'];
	const ordinaryFile = path.join(options.evidence, 'projection.ordinary.json');
	const ordinary = await options.observer.run({
		root: options.root,
		name: 'projection-ordinary',
		arguments: [...args, '--report', ordinaryFile, '--format', 'json'],
		credential: true,
		expectedExit: 0,
	});
	assert.equal(ordinary.captures.length, 13);
	const ordinaryRaw = filesystem.readFileSync(ordinaryFile, 'utf8');
	const plain = byRule(requests(ordinaryRaw), 'independent_document_name')[0];
	assert.ok(plain);
	const cutFields = arrayValue(objectValue(objectValue(plain['request'])['state'])['cut-fields']);
	assert.ok(
		cutFields.includes('selected_sections'),
		'Bounded ordinary plain request did not witness shared sibling cuts',
	);
	const waitingFile = path.join(options.evidence, 'projection.waiting.json');
	const waiting = await options.observer.run({
		root: options.root,
		name: 'projection-waiting',
		arguments: [...args, '--prepare', waitingFile],
		expectedExit: options.prepareExit,
	});
	assert.equal(waiting.captures.length, 0);
	const waitingRaw = filesystem.readFileSync(waitingFile, 'utf8');
	requireSelectionWaits(waitingRaw);
	const pending = requests(waitingRaw);
	assert.equal(byRule(pending, 'composed_sections').length, 0);
	assert.equal(
		byRule(pending, 'independent_document_name').length,
		0,
		'Unknown sibling fields fabricated the shared model-view cut metadata',
	);
	assert.equal(
		byRule(pending, 'independent_heading').length,
		4,
		'Projection uncertainty suppressed unrelated kind',
	);
	assert.equal(preparedInventory(waitingRaw).size, 5);
	const replayFile = path.join(options.evidence, 'projection.replayed.json');
	const replay = await options.observer.run({
		root: options.root,
		name: 'projection-replayed',
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
		expectedPrepared: 13,
	});
}
export async function observeReviewControls(options: {
	observer: RulebookProcess;
	evidence: string;
	source: string;
	prepareExit: number;
}): Promise<void> {
	const { observer, evidence } = options;
	observer.respond = ({ body }) => Promise.resolve({ status: 200, body: responseBody(body, 0.95) });
	const composedRoot = path.join(evidence, 'composed-selection-fixture');
	composedSelectionFixture(composedRoot, options.source);
	configureEndpoint(composedRoot, observer.endpoint);
	const possibleChars =
		Array.from(filesystem.readFileSync(path.join(composedRoot, researchFile), 'utf8')).length +
		Array.from('practices-research').length;
	assert.ok(
		possibleChars < 6000,
		'Wide independent-field fixture can activate shared projection cuts',
	);
	const configArguments = ['lint', '--config', prepareConfig];
	await observer.run({
		root: composedRoot,
		name: 'composed-check',
		arguments: [...configArguments, '--check'],
		expectedExit: 0,
	});
	const waitingFile = path.join(evidence, 'composed.waiting.json');
	const waitingRun = await observer.run({
		root: composedRoot,
		name: 'composed-waiting',
		arguments: [...configArguments, '--all', '--prepare', waitingFile],
		expectedExit: options.prepareExit,
	});
	assert.equal(waitingRun.captures.length, 0);
	const waitingRaw = filesystem.readFileSync(waitingFile, 'utf8');
	requireSelectionWaits(waitingRaw);
	const pending = requests(waitingRaw);
	assert.equal(
		byRule(pending, 'composed_sections').length,
		0,
		'Deferred selected context fabricated an empty composed request',
	);
	assert.equal(
		byRule(pending, 'independent_heading').length,
		4,
		'Independent kind stopped progressing',
	);
	assert.equal(
		byRule(pending, 'independent_document_name').length,
		1,
		'Independent field on composed kind stopped progressing',
	);
	assert.ok(
		arrayValue(parseObject(waitingRaw)['needs_answer'])
			.map(objectValue)
			.some((entry) => entry['kind'] === 'z_composed'),
	);
	assert.equal(preparedInventory(waitingRaw).size, 6);
	const ordinaryFile = path.join(evidence, 'composed.ordinary.json');
	const normal = await observer.run({
		root: composedRoot,
		name: 'composed-ordinary',
		arguments: [...configArguments, '--all', '--report', ordinaryFile, '--format', 'json'],
		credential: true,
		expectedExit: 0,
	});
	assert.equal(normal.captures.length, 13);
	const ordinaryRaw = filesystem.readFileSync(ordinaryFile, 'utf8');
	const ordinaryRequests = requests(ordinaryRaw);
	const widePlain = byRule(ordinaryRequests, 'independent_document_name')[0];
	assert.ok(widePlain);
	assert.ok(
		!('cut-fields' in objectValue(objectValue(widePlain['request'])['state'])),
		'Wide ordinary independent-field control unexpectedly shares a cut budget',
	);
	const composed = byRule(ordinaryRequests, 'composed_sections');
	assert.equal(composed.length, 1);
	assert.ok(
		JSON.stringify(composed).includes('Frame one to five researchable questions.'),
		'Ordinary composed evidence witness is absent',
	);
	const replayFile = path.join(evidence, 'composed.replayed.json');
	const replay = await observer.run({
		root: composedRoot,
		name: 'composed-replayed',
		arguments: [
			...configArguments,
			'--all',
			'--prepare',
			replayFile,
			'--selections-from',
			ordinaryFile,
		],
		expectedExit: options.prepareExit,
	});
	assert.equal(replay.captures.length, 0);
	const replayRaw = filesystem.readFileSync(replayFile, 'utf8');
	assert.deepEqual(parseObject(replayRaw)['needs_answer'], []);
	requireRequestParity({
		prepared: replayRaw,
		ordinary: ordinaryRaw,
		replayedFrom: ordinaryRaw,
		captures: normal.captures.map((capture) => capture.raw.toString('utf8')),
		expectedPrepared: 13,
	});

	await observeProjectionDependency({
		observer,
		evidence,
		root: composedRoot,
		prepareExit: options.prepareExit,
	});
	const budgetRoot = path.join(evidence, 'failed-admission-fixture');
	failedAdmissionFixture(budgetRoot);
	configureEndpoint(budgetRoot, observer.endpoint);
	await observer.run({
		root: budgetRoot,
		name: 'failed-admission-check',
		arguments: [...configArguments, '--check'],
		expectedExit: 0,
	});
	await runSequence([2, 1], async (allowance) => {
		const configPath = path.join(budgetRoot, prepareConfig);
		const config = readObject(configPath);
		objectValue(config['limits'])['max_model_requests'] = allowance;
		writeJson(configPath, config);
		const reportFile = path.join(evidence, `failed-admission-${allowance}.ordinary.json`);
		const budgetOrdinary = await observer.run({
			root: budgetRoot,
			name: `failed-admission-${allowance}-ordinary`,
			arguments: [...configArguments, '--all', '--report', reportFile, '--format', 'json'],
			credential: true,
			expectedExit: 3,
		});
		assert.equal(budgetOrdinary.captures.length, allowance === 2 ? 1 : 0);
		const preparedFile = path.join(evidence, `failed-admission-${allowance}.prepared.json`);
		const prepare = await observer.run({
			root: budgetRoot,
			name: `failed-admission-${allowance}-prepare`,
			arguments: [...configArguments, '--all', '--prepare', preparedFile],
			expectedExit: 3,
		});
		assert.equal(prepare.captures.length, 0);
		const ordinary = filesystem.readFileSync(reportFile, 'utf8');
		const prepared = filesystem.readFileSync(preparedFile, 'utf8');
		const ordinaryEntries = requests(ordinary);
		const preparedEntries = requests(prepared);
		const firstOrdinary = byRule(ordinaryEntries, 'a_oversized');
		const firstPrepared = byRule(preparedEntries, 'a_oversized');
		assert.equal(firstOrdinary.length, 1);
		assert.equal(firstPrepared.length, 1);
		const firstRecord = firstOrdinary[0];
		assert.ok(firstRecord);
		assert.ok(
			JSON.stringify(objectValue(firstRecord['request'])).length < 49000,
			'Oversized-token witness exceeded the outer character cap',
		);
		assert.ok(
			JSON.stringify(objectValue(firstRecord['outcome'])['diagnostics']).includes(
				'context_budget_exceeded',
			),
			'First request failed something other than configured token admission',
		);
		assert.ok(
			JSON.stringify(firstOrdinary).includes('invalid_request'),
			'First ordinary request did not fail local admission',
		);
		assert.ok(
			JSON.stringify(firstPrepared).includes('invalid_request'),
			'First prepared request did not fail local admission',
		);
		const secondOrdinary = byRule(ordinaryEntries, 'z_valid');
		const secondPrepared = byRule(preparedEntries, 'z_valid');
		assert.equal(secondOrdinary.length, 1);
		assert.equal(secondPrepared.length, 1);
		if (allowance === 1) {
			for (const entries of [secondOrdinary, secondPrepared]) {
				assert.equal(entries[0]?.['state'], 'not_sent');
				assert.ok(
					JSON.stringify(entries).includes('request_limit_reached'),
					'Failed admission did not reserve ordinary request allowance',
				);
			}
			assert.ok(preparedEntries.every((entry) => entry['status'] !== 'prepared'));
		} else {
			const preparedIndex = preparedEntries.findIndex(
				(entry) => objectValue(entry['owner'])['rule'] === 'z_valid',
			);
			const ordinaryIndex = ordinaryEntries.findIndex(
				(entry) => objectValue(entry['owner'])['rule'] === 'z_valid',
			);
			assert.equal(preparedEntries[preparedIndex]?.['status'], 'prepared');
			const capture = budgetOrdinary.captures[0];
			assert.ok(capture);
			for (const field of ['state', 'questions']) {
				requirePayloadBytes({
					left: prepared,
					leftPath: ['requests', preparedIndex, 'request', field],
					right: ordinary,
					rightPath: ['requests', ordinaryIndex, 'request', field],
					label: `Valid second ${field} differs`,
				});
				requirePayloadBytes({
					left: ordinary,
					leftPath: ['requests', ordinaryIndex, 'request', field],
					right: capture.raw.toString('utf8'),
					rightPath: [field],
					label: `Second ${field} differs from actual HTTP`,
				});
			}
		}
	});
	writeJson(path.join(evidence, 'review-controls-observed.json'), {
		composed_selection: {
			deferred_composed: true,
			independent_kind: 4,
			independent_field: 1,
			replay_payloads: 13,
			projection_budget: { wide: 6000, bounded: 1000, plain_waits_under_bounded_view: true },
		},
		failed_admission: {
			allowance_two_actual_http: 1,
			allowance_one_actual_http: 0,
			second_refused_in_both_modes: true,
		},
		no_prepare_http: true,
	});
}
