/** Exact-score corpus preparation, deliberately separate from missing ordinary parity. */
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
import { type RulebookProcess } from '../rulebook/rulebook-process.ts';
import { readCorpusSelections, scoreSelectionArguments } from './case-dispatch-corpus.ts';

/** Caller declares the current rulebook config and expected prepared exit on its pinned target. */
export async function observeCorpusSelectedPreparation(options: {
	readonly observer: RulebookProcess;
	readonly evidence: string;
	readonly manifest: string;
	readonly copies: string;
	readonly rulebookConfig: string;
	readonly preparedExit: number;
}): Promise<void> {
	assert.ok(path.isAbsolute(options.rulebookConfig));
	const selections = readCorpusSelections(options.manifest, options.copies);
	const cwd = path.join(options.evidence, 'corpus-driver-cwd');
	filesystem.mkdirSync(cwd);
	const observations: object[] = [];
	await runSequence([...selections.entries()], async ([index, selection]) => {
		const saved = path.join(options.evidence, `corpus-selected-${index + 1}.json`);
		const result = await options.observer.run({
			root: cwd,
			name: `corpus-selected-${index + 1}`,
			arguments: [
				'lint',
				'--config',
				options.rulebookConfig,
				'cases',
				path.join(options.copies, selection.file),
				...scoreSelectionArguments(selection),
				'--prepare',
				saved,
			],
			expectedExit: options.preparedExit,
		});
		assert.equal(result.captures.length, 0, 'Corpus preparation dispatched HTTP');
		const report = parseObject(filesystem.readFileSync(saved, 'utf8'));
		assert.equal(report['mode'], 'prepare');
		assert.deepEqual(objectValue(report['errors']), {}, 'Selected corpus revisions failed');
		const rows = arrayValue(report['cases']).map(objectValue);
		assert.deepEqual(
			rows.map((row) => textValue(row['id'])).toSorted(),
			[...selection.scoreIds].toSorted(),
			'Selected corpus denominator differs',
		);
		const copy = parseObject(
			filesystem.readFileSync(path.join(options.copies, selection.file), 'utf8'),
		);
		const score = new Set(selection.scoreIds);
		const excluded = arrayValue(copy['cases'])
			.map(objectValue)
			.map((row) => textValue(row['id']))
			.filter((id) => !score.has(id));
		assert.deepEqual(
			arrayValue(report['not_selected']).map(textValue).toSorted(),
			excluded.toSorted(),
			'Unselected corpus denominator differs',
		);
		const dispositions = rows.map((row) => ({
			id: textValue(row['id']),
			kind: textValue(objectValue(row['outcome'])['kind']),
		}));
		observations.push({
			...selection,
			dispositions,
			runs: arrayValue(report['runs']).length,
			status: report['status'],
		});
	});
	writeJson(path.join(options.evidence, 'corpus-selected-inventory.json'), {
		observations,
		parity: 'not_checked',
		ordinaryReference: 'not_run',
	});
}
