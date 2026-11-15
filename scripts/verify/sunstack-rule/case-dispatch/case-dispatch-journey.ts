/** Same-rule CLI selection control, composed with the existing owned process observer. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { jsonSlice } from '../prepare/raw-json.ts';
import {
	arrayValue,
	objectValue,
	parseObject,
	textValue,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import { type RulebookProcess } from '../rulebook/rulebook-process.ts';
import {
	attributeControlledCaptures,
	requireCaseAccounting,
	requireCaseRunStatus,
	requireControlledDispatch,
	requirePreparedRuntimeParity,
	requireSelectedRequestParity,
	requireUnassessedBars,
} from './case-dispatch-checks.ts';
import { createCaseDispatchFixture } from './case-dispatch-fixture.ts';

/** Caller owns a started, target-pinned observer and a new external evidence directory. */
export async function observeSameFileCaseDispatch(options: {
	readonly observer: RulebookProcess;
	readonly evidence: string;
}): Promise<void> {
	const root = path.join(options.evidence, 'case-dispatch-fixture');
	const fixture = createCaseDispatchFixture(root, options.observer.endpoint);
	const ordinaryPrepare = path.join(options.evidence, 'ordinary-prepared.json');
	const selectedPrepare = path.join(options.evidence, 'selected-cases-prepared.json');
	const ordinaryReport = path.join(options.evidence, 'ordinary-run.json');
	const selectedReport = path.join(options.evidence, 'selected-cases-run.json');
	const ordinary = ['lint', '--config', 'sunstack.lint.json', '--all'];
	const selected = [
		'lint',
		'--config',
		'sunstack.lint.json',
		'cases',
		fixture.casesPath,
		'--set',
		'development',
		'--case',
		'selected-a',
		'--case',
		'selected-b',
	];
	const selectedOwner = JSON.stringify(['rule', 'approval', 'docs/pieces.md', '1', '1']);
	const excludedOwner = JSON.stringify(['rule', 'approval', 'docs/pieces.md', '3', '3']);
	const preparedOrdinary = await options.observer.run({
		root,
		name: 'case-ordinary-prepare',
		arguments: [...ordinary, '--prepare', ordinaryPrepare],
		expectedExit: 6,
	});
	const preparedSelected = await options.observer.run({
		root,
		name: 'case-selected-prepare',
		arguments: [...selected, '--prepare', selectedPrepare],
		expectedExit: 6,
	});
	assert.equal(preparedOrdinary.captures.length, 0, 'Ordinary prepare sent HTTP');
	assert.equal(preparedSelected.captures.length, 0, 'Selected prepare sent HTTP');
	const baseline = filesystem.readFileSync(ordinaryPrepare, 'utf8');
	const casesRaw = filesystem.readFileSync(selectedPrepare, 'utf8');
	const casesReport = parseObject(casesRaw);
	assert.equal(arrayValue(casesReport['runs']).length, 1, 'Expected one selected revision');
	const selectedRaw = jsonSlice(casesRaw, ['runs', 0, 'report']);
	requireCaseAccounting({
		selectedIds: ['selected-a', 'selected-b'],
		dispositions: arrayValue(casesReport['cases'])
			.map(objectValue)
			.map((row) => ({
				id: textValue(row['id']),
				kind: textValue(objectValue(row['outcome'])['kind']),
			})),
		expectedKinds: new Map([
			['selected-a', 'prepared'],
			['selected-b', 'prepared'],
		]),
	});
	assert.deepEqual(arrayValue(casesReport['not_selected']), ['excluded']);
	const unknown = await options.observer.run({
		root,
		name: 'case-unknown-id',
		arguments: [
			'lint',
			'--config',
			'sunstack.lint.json',
			'cases',
			fixture.casesPath,
			'--set',
			'development',
			'--case',
			'missing-id',
		],
		expectedExit: 2,
	});
	assert.equal(unknown.captures.length, 0);
	assert.match(unknown.stderr.toString('utf8'), /unknown case id/u);
	const duplicatePath = path.join(options.evidence, 'case-duplicate-input.json');
	const duplicateInput = parseObject(filesystem.readFileSync(fixture.casesPath, 'utf8'));
	const duplicateRows = arrayValue(duplicateInput['cases']);
	assert.ok(duplicateRows[0]);
	duplicateRows.push(duplicateRows[0]);
	writeJson(duplicatePath, duplicateInput);
	const duplicate = await options.observer.run({
		root,
		name: 'case-duplicate-id',
		arguments: ['lint', '--config', 'sunstack.lint.json', 'cases', duplicatePath],
		expectedExit: 2,
	});
	assert.equal(duplicate.captures.length, 0);
	assert.match(duplicate.stderr.toString('utf8'), /case ids must be nonempty and unique/u);
	const caseRows = arrayValue(casesReport['cases']).map(objectValue);
	const requestRows = arrayValue(parseObject(selectedRaw)['requests']).map(objectValue);
	assert.equal(requestRows.length, 1, 'Shared owner generated multiple prepared requests');
	for (const row of caseRows)
		assert.equal(
			row['result_id'],
			requestRows[0]?.['id'],
			'Case does not reference its shared request',
		);
	requireSelectedRequestParity({
		ordinary: baseline,
		selected: selectedRaw,
		selectedOwners: [selectedOwner],
		excludedOwners: [excludedOwner],
		prerequisiteOwners: [],
	});
	const executedOrdinary = await options.observer.run({
		root,
		name: 'case-ordinary-run',
		arguments: [...ordinary, '--report', ordinaryReport],
		credential: true,
		expectedExit: 0,
	});
	const executedSelected = await options.observer.run({
		root,
		name: 'case-selected-run',
		arguments: [
			'lint',
			'--config',
			'sunstack.lint.json',
			'--report',
			selectedReport,
			...selected.slice(3),
		],
		credential: true,
		expectedExit: 1,
	});
	const selectedRunRaw = filesystem.readFileSync(selectedReport, 'utf8');
	requireUnassessedBars(parseObject(selectedRunRaw));
	requireCaseRunStatus(parseObject(selectedRunRaw), 'clean', 1);
	const ordinaryRunRaw = filesystem.readFileSync(ordinaryReport, 'utf8');
	const selectedRuntime = jsonSlice(selectedRunRaw, ['runs', 0, 'report']);
	requirePreparedRuntimeParity(baseline, ordinaryRunRaw, [selectedOwner, excludedOwner]);
	requirePreparedRuntimeParity(selectedRaw, selectedRuntime, [selectedOwner]);
	assert.equal(
		arrayValue(parseObject(selectedRunRaw)['runs']).length,
		1,
		'Expected one executed revision',
	);
	requireCaseAccounting({
		selectedIds: ['selected-a', 'selected-b'],
		dispositions: arrayValue(parseObject(selectedRunRaw)['cases'])
			.map(objectValue)
			.map((row) => ({
				id: textValue(row['id']),
				kind: textValue(objectValue(row['outcome'])['kind']),
			})),
		expectedKinds: new Map([
			['selected-a', 'right'],
			['selected-b', 'right'],
		]),
	});
	requireControlledDispatch({
		ordinary: attributeControlledCaptures(
			ordinaryRunRaw,
			executedOrdinary.captures.map((capture) => capture.raw.toString('utf8')),
		),
		selected: attributeControlledCaptures(
			selectedRuntime,
			executedSelected.captures.map((capture) => capture.raw.toString('utf8')),
		),
		selectedOwners: [selectedOwner],
		excludedOwners: [excludedOwner],
		prerequisiteOwners: [],
	});
	const cancelledReport = path.join(options.evidence, 'selected-cases-cancelled.json');
	const cancelled = await options.observer.run({
		root,
		name: 'case-selected-cancelled',
		arguments: [
			'lint',
			'--config',
			'sunstack.lint.json',
			'--report',
			cancelledReport,
			...selected.slice(3),
		],
		credential: true,
		cancel: true,
		expectedExit: 4,
	});
	assert.equal(
		cancelled.captures.length,
		1,
		'Cancellation control did not reach its selected request',
	);
	const cancelledCaseReport = parseObject(filesystem.readFileSync(cancelledReport, 'utf8'));
	requireUnassessedBars(cancelledCaseReport);
	const cancelledRows = arrayValue(cancelledCaseReport['cases']).map(objectValue);
	requireCaseAccounting({
		selectedIds: ['selected-a', 'selected-b'],
		dispositions: cancelledRows.map((row) => ({
			id: textValue(row['id']),
			kind: textValue(objectValue(row['outcome'])['kind']),
		})),
		expectedKinds: new Map([
			['selected-a', 'cancelled'],
			['selected-b', 'cancelled'],
		]),
	});
}
