/** Real CLI same-line byte binding, shared by candidate and release observations. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { compactJson, jsonSlice } from '../prepare/raw-json.ts';
import { arrayValue, objectValue, parseObject } from '../rulebook/rulebook-fixture-support.ts';
import type { RulebookProcess } from '../rulebook/rulebook-process.ts';
import { createArchiveObserver, requireArchiveRevisions } from './archive-observer.ts';
import { createByteFixture } from './byte-fixture.ts';
import { requireByteCaseDetail, requirePreparedByteLinks } from './byte-observation.ts';

function preparedPayload(raw: string, id: string): string {
	const report = parseObject(raw);
	const cases = arrayValue(report['cases']).map(objectValue);
	const found = cases.find((item) => item['id'] === id);
	assert.ok(found, `Missing prepared case ${id}`);
	const runs = arrayValue(report['runs']);
	assert.equal(runs.length, 1);
	const requests = arrayValue(objectValue(objectValue(runs[0])['report'])['requests']).map(
		objectValue,
	);
	const index = requests.findIndex((request) => request['id'] === found['result_id']);
	assert.ok(index >= 0, `Missing prepared request for ${id}`);
	return compactJson(jsonSlice(raw, ['runs', 0, 'report', 'requests', index, 'request']));
}

function scoredPayload(raw: string, id: string): string {
	const report = parseObject(raw);
	const found = arrayValue(report['cases'])
		.map(objectValue)
		.find((item) => item['id'] === id);
	assert.ok(found, `Missing scored case ${id}`);
	const runs = arrayValue(report['runs']);
	assert.equal(runs.length, 1);
	const nested = objectValue(objectValue(runs[0])['report']);
	const result = arrayValue(nested['results'])
		.map(objectValue)
		.find((item) => item['id'] === found['result_id']);
	assert.ok(result, `Missing scored result for ${id}`);
	const ids = arrayValue(result['request_ids']);
	assert.equal(ids.length, 1);
	const requests = arrayValue(nested['requests']).map(objectValue);
	const index = requests.findIndex((request) => request['id'] === ids[0]);
	assert.ok(index >= 0, `Missing scored request for ${id}`);
	return compactJson(jsonSlice(raw, ['runs', 0, 'report', 'requests', index, 'request']));
}

export async function observeByteCliJourney(
	observer: RulebookProcess,
	evidence: string,
): Promise<{
	readonly root: string;
	readonly revision: string;
	readonly unselectedRevision: string;
}> {
	const root = path.join(evidence, 'fixture');
	const fixture = createByteFixture(root, observer.endpoint);
	const archive = createArchiveObserver(path.join(evidence, 'git-forwarder'), '/usr/bin/git');
	const preparedPath = path.join(evidence, 'prepared.json');
	const scoredPath = path.join(evidence, 'scored.json');
	const selected = [
		'lint',
		'--config',
		'sunstack.lint.json',
		'cases',
		fixture.casesPath,
		'--set',
		'development',
		'--case',
		'first',
		'--case',
		'second',
	];
	const prepared = await observer.run({
		root,
		name: 'both-prepare',
		executableDirectory: archive.executableDirectory,
		arguments: [...selected, '--prepare', preparedPath],
		expectedExit: 6,
	});
	assert.equal(prepared.captures.length, 0, 'Prepare dispatched a request');
	requireArchiveRevisions(archive.archiveLog, [fixture.revision]);
	assert.notEqual(fixture.revision, fixture.unselectedRevision);
	requirePreparedByteLinks(parseObject(filesystem.readFileSync(preparedPath, 'utf8')), [
		'first',
		'second',
	]);
	const scored = await observer.run({
		root,
		name: 'both-score',
		arguments: [
			'lint',
			'--config',
			'sunstack.lint.json',
			'--report',
			scoredPath,
			...selected.slice(3),
		],
		credential: true,
		expectedExit: 1,
	});
	assert.equal(scored.captures.length, 2, 'Scoring did not dispatch two distinct requests');
	const report = parseObject(filesystem.readFileSync(scoredPath, 'utf8'));
	const cases = arrayValue(report['cases']).map(objectValue);
	assert.deepEqual(
		cases.map((item) => [item['id'], objectValue(item['outcome'])['kind']]),
		[
			['first', 'right'],
			['second', 'right'],
		],
	);
	const bothPrepared = filesystem.readFileSync(preparedPath, 'utf8');
	const observeOne = async (id: string, expectedText: string): Promise<void> => {
		const onePreparedPath = path.join(evidence, `prepared-${id}.json`);
		const oneScoredPath = path.join(evidence, `scored-${id}.json`);
		const args = [
			'lint',
			'--config',
			'sunstack.lint.json',
			'cases',
			fixture.casesPath,
			'--set',
			'development',
			'--case',
			id,
		];
		const preparedOne = await observer.run({
			root,
			name: `prepare-${id}`,
			arguments: [...args, '--prepare', onePreparedPath],
			expectedExit: 6,
		});
		assert.equal(preparedOne.captures.length, 0);
		const raw = filesystem.readFileSync(onePreparedPath, 'utf8');
		requirePreparedByteLinks(parseObject(raw), [id]);
		assert.equal(
			preparedPayload(raw, id),
			preparedPayload(bothPrepared, id),
			'Selected prepared payload differs',
		);
		const scoredOne = await observer.run({
			root,
			name: `score-${id}`,
			arguments: [
				'lint',
				'--config',
				'sunstack.lint.json',
				'--report',
				oneScoredPath,
				...args.slice(3),
			],
			credential: true,
			expectedExit: 1,
		});
		assert.equal(scoredOne.captures.length, 1, 'One selected case must dispatch one request');
		const reference = scored.captures.filter(
			(capture) => objectValue(capture.body['state'])['body'] === expectedText,
		);
		assert.equal(reference.length, 1, 'Both-case run did not identify one matching request');
		assert.deepEqual(
			scoredOne.captures[0]?.raw,
			reference[0]?.raw,
			'Selected request payload differs',
		);
		const scoredRaw = filesystem.readFileSync(oneScoredPath, 'utf8');
		assert.equal(
			scoredPayload(scoredRaw, id),
			preparedPayload(raw, id),
			'Prepared and scored request payloads differ',
		);
		const result = arrayValue(parseObject(scoredRaw)['cases']).map(objectValue);
		assert.deepEqual(
			result.map((item) => [item['id'], objectValue(item['outcome'])['kind']]),
			[[id, 'right']],
		);
	};
	await observeOne('first', 'café yes');
	await observeOne('second', 'no');
	const showCase = async (mode: string, file: string, id: string): Promise<void> => {
		const shown = await observer.run({
			root,
			name: `show-${mode}-${id}`,
			arguments: ['lint', 'show', file, id],
			expectedExit: 0,
		});
		const detail = parseObject(shown.stdout.toString('utf8'));
		requireByteCaseDetail(detail, id, fixture.revision);
	};
	await showCase('prepared', preparedPath, 'first');
	await showCase('prepared', preparedPath, 'second');
	await showCase('scored', scoredPath, 'first');
	await showCase('scored', scoredPath, 'second');
	return { root, revision: fixture.revision, unselectedRevision: fixture.unselectedRevision };
}
