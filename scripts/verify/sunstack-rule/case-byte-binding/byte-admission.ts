/** File-wide geometry and all-selected archived bounds refusal controls. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	objectValue,
	parseObject,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import type { RulebookProcess } from '../rulebook/rulebook-process.ts';
import { createArchiveObserver, requireArchiveRevisions } from './archive-observer.ts';
import { byteFixtureCases, byteFixturePath } from './byte-fixture.ts';

export async function observeByteAdmission(options: {
	readonly observer: RulebookProcess;
	readonly root: string;
	readonly evidence: string;
	readonly firstRevision: string;
	readonly secondRevision: string;
}): Promise<void> {
	const common = {
		...byteFixtureCases[0],
		kind: 'rule',
		rule: 'approval',
		file: byteFixturePath,
		revision: options.firstRevision,
		lines: [1, 1],
		set: 'development',
		label_from: 'blind-agents',
	};
	const valid = { ...common, id: 'valid' };
	const refuseShape = async (name: string, bytes: readonly (number | string)[]): Promise<void> => {
		const input = path.join(options.evidence, `bad-shape-${name}.json`);
		writeJson(input, {
			repository: options.root,
			cases: [valid, { ...common, id: 'unselected-invalid', bytes }],
			bars: [],
		});
		if (name === 'overflow') {
			const text = filesystem.readFileSync(input, 'utf8');
			assert.ok(text.includes('"OVERFLOW"'));
			filesystem.writeFileSync(input, text.replace('"OVERFLOW"', '18446744073709551616'));
		}
		const result = await options.observer.run({
			root: options.root,
			name: `bad-shape-${name}`,
			arguments: ['lint', '--config', 'sunstack.lint.json', 'cases', input, '--case', 'valid'],
			credential: true,
			expectedExit: 2,
		});
		assert.equal(result.captures.length, 0, 'Malformed unselected geometry dispatched');
	};
	await refuseShape('short', [0]);
	await refuseShape('negative', [-1, 1]);
	await refuseShape('empty', [4, 4]);
	await refuseShape('reversed', [5, 4]);
	await refuseShape('string', ['0', '16']);
	await refuseShape('overflow', [0, 'OVERFLOW']);
	const unselectedInput = path.join(options.evidence, 'unselected-boundaries.json');
	writeJson(unselectedInput, {
		repository: options.root,
		cases: [
			valid,
			{ ...common, id: 'unselected-bounds', revision: options.secondRevision, bytes: [0, 9999] },
			{ ...common, id: 'unselected-unavailable', revision: 'f'.repeat(40) },
		],
		bars: [],
	});
	const archive = createArchiveObserver(
		path.join(options.evidence, 'unselected-git-forwarder'),
		'/usr/bin/git',
	);
	const selectedOutput = path.join(options.evidence, 'unselected-boundaries-prepared.json');
	const unselected = await options.observer.run({
		root: options.root,
		name: 'unselected-boundaries',
		executableDirectory: archive.executableDirectory,
		arguments: [
			'lint',
			'--config',
			'sunstack.lint.json',
			'cases',
			unselectedInput,
			'--case',
			'valid',
			'--prepare',
			selectedOutput,
		],
		expectedExit: 6,
	});
	assert.equal(unselected.captures.length, 0);
	requireArchiveRevisions(archive.archiveLog, [options.firstRevision]);
	const selectedReport = parseObject(filesystem.readFileSync(selectedOutput, 'utf8'));
	const selectedRows = arrayValue(selectedReport['cases']).map(objectValue);
	assert.equal(selectedRows.length, 1);
	assert.equal(objectValue(selectedRows[0]?.['outcome'])['kind'], 'prepared');
	const boundsInput = path.join(options.evidence, 'bad-selected-bounds.json');
	const [earlier, later] = [options.firstRevision, options.secondRevision].toSorted((left, right) =>
		left.localeCompare(right),
	);
	assert.ok(earlier && later && earlier !== later);
	writeJson(boundsInput, {
		repository: options.root,
		cases: [
			{ ...valid, revision: earlier },
			{ ...common, id: 'later-invalid', revision: later, bytes: [0, 9999] },
		],
		bars: [],
	});
	const preparePath = path.join(options.evidence, 'bad-selected-bounds-prepared.json');
	const result = await options.observer.run({
		root: options.root,
		name: 'bad-selected-bounds',
		arguments: [
			'lint',
			'--config',
			'sunstack.lint.json',
			'cases',
			boundsInput,
			'--prepare',
			preparePath,
		],
		credential: true,
		expectedExit: 2,
	});
	assert.equal(result.captures.length, 0, 'Selected invalid bounds dispatched');
	assert.equal(
		filesystem.existsSync(preparePath),
		false,
		'Refused invocation wrote a prepared case report',
	);
}
