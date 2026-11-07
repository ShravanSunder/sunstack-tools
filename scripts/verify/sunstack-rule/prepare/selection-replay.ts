/** Continue only from recorded selection answers, then reject stale or unanswered inputs. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	objectValue,
	parseObject,
	runSequence,
	textValue,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { type CapturedRequest, type RulebookProcess } from '../rulebook/rulebook-process.ts';
import { researchFile } from '../selection/selection-fixture.ts';
import { requestOwner, requireRequestParity, requireSelectionWaits } from './prepare-checks.ts';
import { prepareConfig } from './prepare-fixtures.ts';
import { requirePayloadBytes } from './raw-json.ts';

function requireUnresolved(raw: string): void {
	requireSelectionWaits(raw);
	const requests = arrayValue(parseObject(raw)['requests']).map(objectValue);
	assert.ok(
		requests.every((record) => objectValue(record['owner'])['kind'] === 'selection'),
		'Unmatched selection fabricated downstream requests',
	);
}
function savedRequests(raw: string): JsonObject[] {
	return arrayValue(parseObject(raw)['requests']).map(objectValue);
}
function requireAllowanceRefusal(record: JsonObject): string {
	assert.equal(record['state'], 'not_sent');
	assert.deepEqual(record['reason'], { kind: 'rule', reason: 'request_limit_reached' });
	return requestOwner(record);
}
async function observeReplayAllowance(options: {
	observer: RulebookProcess;
	root: string;
	evidence: string;
	ordinaryFile: string;
}): Promise<void> {
	const sourceRaw = filesystem.readFileSync(options.ordinaryFile, 'utf8');
	const sourceRecords = savedRequests(sourceRaw);
	const sourceIndex = sourceRecords.findIndex(
		(record) => objectValue(record['owner'])['kind'] === 'selection',
	);
	const source = sourceRecords[sourceIndex];
	assert.ok(source);
	assert.equal(
		sourceRecords.filter((record) => objectValue(record['owner'])['kind'] === 'selection').length,
		1,
	);
	assert.equal(
		sourceRecords.filter((record) => objectValue(record['owner'])['kind'] === 'rule').length,
		6,
	);
	const configFile = path.join(options.root, prepareConfig);
	const originalConfig = filesystem.readFileSync(configFile);
	try {
		await runSequence([2, 1], async (allowance) => {
			const config = parseObject(originalConfig.toString('utf8'));
			objectValue(config['limits'])['max_model_requests'] = allowance;
			writeJson(configFile, config);
			const ordinaryFile = path.join(
				options.evidence,
				`replay-allowance-${allowance}.ordinary.json`,
			);
			const ordinary = await options.observer.run({
				root: options.root,
				name: `replay-allowance-${allowance}-ordinary`,
				arguments: [
					'lint',
					'--config',
					prepareConfig,
					'--all',
					'--format',
					'json',
					'--report',
					ordinaryFile,
				],
				credential: true,
				expectedExit: 3,
			});
			assert.equal(
				ordinary.captures.length,
				allowance,
				'Ordinary selector did not occupy one request slot',
			);
			const preparedFile = path.join(
				options.evidence,
				`replay-allowance-${allowance}.prepared.json`,
			);
			const replay = await options.observer.run({
				root: options.root,
				name: `replay-allowance-${allowance}-prepare`,
				arguments: [
					'lint',
					'--config',
					prepareConfig,
					'--all',
					'--prepare',
					preparedFile,
					'--selections-from',
					options.ordinaryFile,
				],
				expectedExit: 3,
			});
			assert.equal(replay.captures.length, 0);
			const ordinaryRaw = filesystem.readFileSync(ordinaryFile, 'utf8');
			const preparedRaw = filesystem.readFileSync(preparedFile, 'utf8');
			const normalReport = parseObject(ordinaryRaw);
			const replayReport = parseObject(preparedRaw);
			assert.equal(objectValue(normalReport['usage'])['model_requests'], allowance);
			assert.equal(objectValue(replayReport['usage'])['model_requests'], 0);
			assert.deepEqual(replayReport['needs_answer'], []);
			const normal = savedRequests(ordinaryRaw);
			const prepared = savedRequests(preparedRaw);
			const selectors = prepared.filter(
				(record) => objectValue(record['owner'])['kind'] === 'selection',
			);
			assert.equal(selectors.length, 1);
			const selector = selectors[0];
			assert.ok(selector);
			assert.equal(selector['state'], 'from_report');
			assert.equal(selector['status'], 'from_report');
			assert.equal(selector['source_request_id'], source['id']);
			assert.equal(selector['resolved_model'], source['resolved_model']);
			assert.equal(selector['input_sha256'], source['input_sha256']);
			const selectorIndex = prepared.indexOf(selector);
			for (const field of ['request', 'outcome', 'selection_evidence'])
				requirePayloadBytes({
					left: preparedRaw,
					leftPath: ['requests', selectorIndex, field],
					right: sourceRaw,
					rightPath: ['requests', sourceIndex, field],
					label: `Allowance replay changed recorded ${field}`,
				});
			const normalRules = normal.filter(
				(record) => objectValue(record['owner'])['kind'] === 'rule',
			);
			const preparedRules = prepared.filter(
				(record) => objectValue(record['owner'])['kind'] === 'rule',
			);
			assert.equal(normalRules.length, 6);
			assert.equal(preparedRules.length, 6);
			assert.equal(new Set(normalRules.map(requestOwner)).size, 6);
			assert.equal(new Set(preparedRules.map(requestOwner)).size, 6);
			assert.deepEqual(
				normalRules.map(requestOwner).toSorted(),
				preparedRules.map(requestOwner).toSorted(),
			);
			const normalRefused = normalRules.filter((record) => record['state'] === 'not_sent');
			const preparedRefused = preparedRules.filter((record) => record['state'] === 'not_sent');
			assert.equal(normalRefused.length, 7 - allowance);
			assert.equal(
				preparedRefused.length,
				7 - allowance,
				'Replayed selector skipped its logical allowance slot',
			);
			assert.deepEqual(
				normalRefused.map(requireAllowanceRefusal).toSorted(),
				preparedRefused.map(requireAllowanceRefusal).toSorted(),
			);
			const offered = preparedRules.filter((record) => record['status'] === 'prepared');
			const sent = normalRules.filter((record) => record['state'] === 'run');
			assert.equal(offered.length, allowance - 1);
			assert.equal(sent.length, allowance - 1);
			assert.deepEqual(offered.map(requestOwner), sent.map(requestOwner));
			for (const record of offered) {
				assert.equal(record['state'], 'prepared');
				assert.ok(!('outcome' in record));
				const normalRecord = sent.find((item) => requestOwner(item) === requestOwner(record));
				assert.ok(normalRecord);
				assert.equal(objectValue(normalRecord['outcome'])['kind'], 'completed');
				const capture = ordinary.captures[1];
				assert.ok(capture);
				for (const field of ['state', 'questions']) {
					requirePayloadBytes({
						left: preparedRaw,
						leftPath: ['requests', prepared.indexOf(record), 'request', field],
						right: ordinaryRaw,
						rightPath: ['requests', normal.indexOf(normalRecord), 'request', field],
						label: `Admitted replay ${field} differs from ordinary`,
					});
					requirePayloadBytes({
						left: ordinaryRaw,
						leftPath: ['requests', normal.indexOf(normalRecord), 'request', field],
						right: capture.raw.toString('utf8'),
						rightPath: [field],
						label: `Admitted rule ${field} differs from actual HTTP`,
					});
				}
			}
		});
	} finally {
		filesystem.writeFileSync(configFile, originalConfig);
	}
	writeJson(path.join(options.evidence, 'replay-allowance-observed.json'), {
		selector_slots: 1,
		limits: [2, 1],
		ordinary_http: [2, 1],
		prepared_rules: [1, 0],
		refused_rules: [5, 6],
		refused_owners_match: true,
		no_prepare_http: true,
		replay_model_usage: 0,
	});
}
export async function observeSelectionReplay(options: {
	observer: RulebookProcess;
	root: string;
	evidence: string;
	ordinaryFile: string;
	unansweredFile: string;
	captures: readonly CapturedRequest[];
	prepareExit: number;
}): Promise<void> {
	const ordinary = filesystem.readFileSync(options.ordinaryFile, 'utf8');
	const records = arrayValue(parseObject(ordinary)['requests']).map(objectValue);
	const selectors = records.filter(
		(record) => objectValue(record['owner'])['kind'] === 'selection',
	);
	assert.ok(selectors.length > 0);
	assert.ok(
		records.length > selectors.length,
		'Ordinary selection produced no downstream requests',
	);
	for (const record of selectors) {
		assert.equal(record['state'], 'run');
		assert.match(textValue(record['input_sha256']), /^[a-f0-9]{64}$/);
		const outcome = objectValue(record['outcome']);
		assert.equal(outcome['kind'], 'completed');
		assert.equal(record['resolved_model'], 'typesafe/controlled-rulebook');
		assert.equal(objectValue(outcome['receipt'])['resolved_model'], record['resolved_model']);
		const selectionEvidence = objectValue(record['selection_evidence']);
		assert.ok(arrayValue(selectionEvidence['candidates']).length > 0);
		objectValue(selectionEvidence['shared']);
	}
	async function prepare(name: string, source: string): Promise<string> {
		const output = path.join(options.evidence, `${name}.prepared.json`);
		const run = await options.observer.run({
			root: options.root,
			name,
			arguments: [
				'lint',
				'--config',
				prepareConfig,
				'--prepare',
				output,
				'--selections-from',
				source,
				'--all',
			],
			expectedExit: options.prepareExit,
		});
		assert.equal(run.captures.length, 0, 'Recorded-selection prepare dispatched HTTP');
		const raw = filesystem.readFileSync(output, 'utf8');
		assert.equal(objectValue(parseObject(raw)['usage'])['model_requests'], 0);
		return raw;
	}
	const replay = await prepare('selection-replayed', options.ordinaryFile);
	requireRequestParity({
		prepared: replay,
		ordinary,
		replayedFrom: ordinary,
		captures: options.captures.map((capture) => capture.raw.toString('utf8')),
		expectedPrepared: records.length,
	});
	const replayReport = parseObject(replay);
	assert.deepEqual(replayReport['needs_answer'], []);
	const replayRecords = arrayValue(replayReport['requests']).map(objectValue);
	assert.equal(
		replayRecords.filter((record) => record['state'] === 'from_report').length,
		selectors.length,
	);
	// Original raw candidate evidence, including model views and cuts, must survive replay.
	for (const [index, record] of replayRecords.entries()) {
		if (record['state'] !== 'from_report') continue;
		const unanswered = arrayValue(
			parseObject(filesystem.readFileSync(options.unansweredFile, 'utf8'))['requests'],
		).map(objectValue);
		const sourceIndex = unanswered.findIndex(
			(entry) => JSON.stringify(entry['owner']) === JSON.stringify(record['owner']),
		);
		assert.ok(sourceIndex >= 0);
		requirePayloadBytes({
			left: replay,
			leftPath: ['requests', index, 'selection_evidence'],
			right: filesystem.readFileSync(options.unansweredFile, 'utf8'),
			rightPath: ['requests', sourceIndex, 'selection_evidence'],
			label: 'Original selection evidence changed on replay',
		});
	}

	requireUnresolved(await prepare('selection-unanswered', options.unansweredFile));
	const configFile = path.join(options.root, prepareConfig);
	const candidateFile = path.join(options.root, researchFile);
	const originalConfig = filesystem.readFileSync(configFile);
	const originalCandidate = filesystem.readFileSync(candidateFile);
	await runSequence(['model-alias', 'model-version', 'question', 'candidate'], async (changed) => {
		try {
			if (changed === 'model-alias' || changed === 'model-version') {
				const config = parseObject(originalConfig.toString('utf8'));
				objectValue(config['decision_model'])['model'] =
					changed === 'model-alias' ? 'jev-latest' : 'typesafe/controlled-rulebook-v2';
				writeJson(configFile, config);
			} else if (changed === 'question') {
				const config = parseObject(originalConfig.toString('utf8'));
				const kinds = objectValue(objectValue(config['evidence'])['kinds']);
				const question = objectValue(
					objectValue(objectValue(kinds['workflow_section'])['select'])['question'],
				);
				objectValue(question['instructions'])['task'] =
					'CHANGED_QUESTION_SENTINEL: apply a different controlled standard.';
				writeJson(configFile, config);
			} else {
				const candidate = originalCandidate.toString('utf8');
				const changedCandidate = candidate.replace(
					'Frame one to five researchable questions.',
					'CHANGED_CANDIDATE_SENTINEL: Frame one to five researchable questions.',
				);
				assert.notEqual(changedCandidate, candidate, 'Changed-candidate witness was inert');
				filesystem.writeFileSync(candidateFile, changedCandidate);
			}
			requireUnresolved(await prepare(`selection-changed-${changed}`, options.ordinaryFile));
		} finally {
			filesystem.writeFileSync(configFile, originalConfig);
			filesystem.writeFileSync(candidateFile, originalCandidate);
		}
	});
	await observeReplayAllowance(options);
	writeJson(path.join(options.evidence, 'selection-replay-observed.json'), {
		selectors: selectors.length,
		downstream: records.length - selectors.length,
		byte_parity: true,
		zero_http: true,
		unresolved_controls: [
			'unanswered',
			'model-alias',
			'model-version',
			'changed-question',
			'changed-candidate',
		],
	});
}
