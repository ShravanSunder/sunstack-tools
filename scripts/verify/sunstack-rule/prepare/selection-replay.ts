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
} from '../rulebook/rulebook-fixture-support.ts';
import { type CapturedRequest, type RulebookProcess } from '../rulebook/rulebook-process.ts';
import { researchFile } from '../selection/selection-fixture.ts';
import { requireRequestParity, requireSelectionWaits } from './prepare-checks.ts';
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
