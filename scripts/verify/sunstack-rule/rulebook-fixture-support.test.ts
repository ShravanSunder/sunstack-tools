/** Sensitivity checks for verification comparators and actual child/HTTP boundaries. */
import filesystem from 'node:fs';
import operatingSystem from 'node:os';
import path from 'node:path';

import { afterEach, expect, test } from 'vitest';

import { lintCaseSchema, requireReport } from './lint-runner-rulebook.ts';
import {
	listingPieces,
	requireRepeatedRequests,
	requireRequestPiece,
} from './rulebook-evidence-checks.ts';
import {
	arrayValue,
	baselineGit,
	configName,
	gitCommand,
	objectValue,
	parseObject,
	readObject,
	skillName,
	type JsonObject,
	type JsonValue,
} from './rulebook-fixture-support.ts';
import { confirmTarget, declaredTarget, RulebookProcess } from './rulebook-process.ts';
import { prepareCases } from './rulebook-refusal-fixtures.ts';

const temporary: string[] = [];
function scratch(): string {
	const root = filesystem.mkdtempSync(path.join(operatingSystem.tmpdir(), 'rulebook-apparatus-'));
	temporary.push(root);
	return root;
}
afterEach(() => {
	for (const root of temporary.splice(0)) filesystem.rmSync(root, { recursive: true, force: true });
});
function errorReport(): JsonObject {
	return {
		status: { kind: 'findings' },
		findings: [
			{
				marker: 'K002',
				file: skillName,
				line: 3,
				severity: 'error',
				message: 'Over 300 characters.',
			},
		],
		usage: { model_requests: 0 },
		counts: { error: 1, warn: 0 },
	};
}
const findingMutations: [string, JsonValue][] = [
	['marker', 'K003'],
	['file', 'wrong.md'],
	['severity', 'warn'],
	['line', 4],
	['message', ''],
];
test.each(findingMutations)('comparator rejects wrong finding %s', (field, value) => {
	const scenario = lintCaseSchema.parse({
		name: 'mechanical-error',
		exit: 1,
		status: 'findings',
		severity: 'error',
	});
	const report = errorReport();
	requireReport(report, scenario);
	objectValue(arrayValue(report['findings'])[0])[field] = value;
	expect(() => requireReport(report, scenario)).toThrow();
});
test('refusal setup preserves duplicate/malformed inputs and all adopted controls', () => {
	const root = path.join(scratch(), 'cases');
	const cases = prepareCases(root);
	expect(
		filesystem
			.readFileSync(path.join(root, 'config-duplicate', configName), 'utf8')
			.match(/"marker_prefix"/g),
	).toHaveLength(2);
	expect(() =>
		parseObject(filesystem.readFileSync(path.join(root, 'config-syntax', configName), 'utf8')),
	).toThrow();
	expect(new Set(cases.map((entry) => entry.name)).size).toBe(cases.length);
	expect(cases).toHaveLength(71);
	for (const name of [
		'regex-target',
		'exists-text',
		'as-kind-has-target',
		'threshold-equal',
		'probability-zero',
	])
		expect(cases.some((entry) => entry.name === name)).toBe(true);
	const entry = objectValue(
		readObject(path.join(root, 'choice-too-few/rulebook/steering.rules.json'))['K001'],
	);
	expect(objectValue(objectValue(entry['questions'])['kind'])['criteria']).toEqual({
		outcome: 'Only one option',
	});
	expect(
		objectValue(objectValue(objectValue(entry['decision'])['steps'])['kind'])['routes'],
	).toEqual({ outcome: 'pass' });
	expect(
		filesystem.existsSync(path.join(root, 'ast-util-invalid/evidence/rules/util-use.yml')),
	).toBe(true);
	expect(
		filesystem.readFileSync(path.join(root, 'marker-spaced-heading/rulebook/steering.md'), 'utf8'),
	).toContain('Extra heading <!-- K004 -->');
});
test('missing key compares acquisitions rather than physical reads', () => {
	const scenario = lintCaseSchema.parse({
		name: 'missing-key',
		exit: 2,
		status: 'invalid',
		marker: 'K003',
	});
	const report: JsonObject = {
		status: { kind: 'invalid' },
		findings: [],
		counts: { error: 0, warn: 0 },
		usage: { model_requests: 0, checked_files_acquired: 0 },
	};
	requireReport(report, scenario);
	objectValue(report['usage'])['checked_files_acquired'] = 1;
	expect(() => requireReport(report, scenario)).toThrow('acquired checked content');
});
test('listing requires the declared envelope', () => {
	const report: JsonObject = {
		version: '0.0.0',
		config: 'fixture.json',
		scope: { kind: 'all' },
		usage: { model_requests: 0 },
		pieces: [],
	};
	expect(listingPieces(Buffer.from(JSON.stringify(report)))).toEqual([]);
	expect(() => listingPieces(Buffer.from('[]'))).toThrow();
	delete report['pieces'];
	expect(() => listingPieces(Buffer.from(JSON.stringify(report)))).toThrow();
});
test('request comparator rejects raw middle, extra piece and changed standard', () => {
	const piece: JsonObject = {
		where: { file: 'one.md', first_line: 1 },
		fields: { body: { state: 'text', value: 'BEGIN_PRIVATE_MIDDLE_END', spans: [] } },
		text: 'BEGIN_PRIVATE_MIDDLE_END',
		model_view: { state: 'bounded', cut: ['body'], fields: { body: 'BEGIN[cut]END' } },
	};
	const state: JsonObject = {
		standard: 'Literal standard.',
		where: piece['where'] ?? null,
		body: 'BEGIN[cut]END',
	};
	requireRequestPiece({ state, piece, standard: 'Literal standard.', questions: {} });
	for (const changed of [
		{ ...state, body: 'BEGIN_PRIVATE_MIDDLE_END' },
		{ ...state, second_piece: { body: 'Another piece' } },
		{ ...state, standard: 'Wrong standard.' },
	])
		expect(() =>
			requireRequestPiece({ state: changed, piece, standard: 'Literal standard.', questions: {} }),
		).toThrow('exactly one piece');
});
test('request comparator omits unreferenced unavailable fields and blocks referenced ones', () => {
	const piece: JsonObject = {
		where: { file: 'one.md' },
		fields: {
			body: { state: 'text', value: 'Visible', spans: [] },
			optional: { state: 'unavailable', reason: 'unreadable' },
		},
		model_view: {
			state: 'bounded',
			cut: [],
			fields: { body: 'Visible', optional: { unavailable: true } },
		},
	};
	const state: JsonObject = {
		standard: 'Standard',
		where: piece['where'] ?? null,
		body: 'Visible',
	};
	const questions: JsonObject = { question: { type: 'noul', instructions: { check: 'body' } } };
	requireRequestPiece({ state, piece, standard: 'Standard', questions });
	expect(() =>
		requireRequestPiece({
			state: { ...state, optional: { unavailable: true } },
			piece,
			standard: 'Standard',
			questions,
		}),
	).toThrow('exactly one piece');
	expect(() =>
		requireRequestPiece({
			state,
			piece,
			standard: 'Standard',
			questions: { question: { type: 'noul', instructions: { context: ['optional'] } } },
		}),
	).toThrow('block dispatch');
});
test('repeat allows arrival reordering and rejects emitted report reordering', () => {
	const first: JsonObject = { standard: 'First', where: { file: 'one.md' } };
	const second: JsonObject = { standard: 'Second', where: { file: 'two.md' } };
	const firstBody = Buffer.from(JSON.stringify({ model: 'fixture', questions: {}, state: first }));
	const secondBody = Buffer.from(
		JSON.stringify({ model: 'fixture', questions: {}, state: second }),
	);
	const report = parseObject(
		`{"evidence_seen":{"0":${JSON.stringify(first)},"1":${JSON.stringify(second)}}}`,
	);
	requireRepeatedRequests({
		first: report,
		second: report,
		firstBodies: [firstBody, secondBody],
		secondBodies: [secondBody, firstBody],
	});
	expect(() =>
		requireRepeatedRequests({
			first: report,
			second: parseObject(
				`{"evidence_seen":{"1":${JSON.stringify(second)},"0":${JSON.stringify(first)}}}`,
			),
			firstBodies: [firstBody, secondBody],
			secondBodies: [firstBody, secondBody],
		}),
	).toThrow('Report request order');
});
function executable(root: string, content: string): string {
	const binary = path.join(root, 'fake-cli');
	filesystem.writeFileSync(binary, `#!${process.execPath}\n${content}`);
	filesystem.chmodSync(binary, 0o755);
	return binary;
}
test('real owned child keeps actual loopback capture', async () => {
	const root = path.join(scratch(), 'source');
	filesystem.mkdirSync(root);
	const binary = executable(
		root,
		`const body={model:'fixture',questions:{names_trigger:{type:'noul',instructions:'Observe'}},state:{standard:'literal'}};const response=await fetch(process.argv[2],{method:'POST',body:JSON.stringify(body)});process.stdout.write(await response.text());`,
	);
	baselineGit(root);
	const target = declaredTarget({
		binary,
		source: root,
		commit: gitCommand(root, ['rev-parse', 'HEAD']),
	});
	const observer = new RulebookProcess(target, path.join(path.dirname(root), 'evidence'));
	await observer.start();
	try {
		const result = await observer.run({
			root,
			name: 'http-child',
			arguments: [observer.endpoint],
			expectedExit: 0,
		});
		expect(result.captures).toHaveLength(1);
		expect(result.captures[0]?.body['state']).toEqual({ standard: 'literal' });
		expect(
			objectValue(
				objectValue(parseObject(result.stdout.toString('utf8'))['answers'])['names_trigger'],
			)['noul'],
		).toBe(0.91);
	} finally {
		await observer.close();
	}
	expect(readObject(path.join(observer.evidence, 'http-child/process.json'))['http_requests']).toBe(
		1,
	);
	expect(readObject(path.join(observer.evidence, 'harness.json'))['errors']).toEqual([]);
});
test('target guard rejects changed executable bytes', () => {
	const root = scratch();
	const binary = executable(root, "process.stdout.write('ready');");
	baselineGit(root);
	const target = declaredTarget({
		binary,
		source: root,
		commit: gitCommand(root, ['rev-parse', 'HEAD']),
	});
	filesystem.appendFileSync(binary, '\n// wrong target');
	expect(() => confirmTarget(target)).toThrow('binary bytes changed');
});
