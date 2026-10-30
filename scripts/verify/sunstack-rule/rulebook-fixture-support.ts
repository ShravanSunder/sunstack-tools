/** Shared fixture preparation and JSON boundaries for the rulebook journeys. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

export const jsonValueSchema = z.json();
export const jsonObjectSchema = z.record(z.string(), jsonValueSchema);
export type JsonValue = z.infer<typeof jsonValueSchema>;
export type JsonObject = z.infer<typeof jsonObjectSchema>;
export const repository = path.resolve(import.meta.dirname, '../../..');
export const fixtureSource = path.join(repository, 'crates/sunstack/tests/fixtures/rulebook-lint');
export const configName = 'skill-rules.config.json';
export const rulesName = 'rulebook/steering.rules.json';
export const rulebookName = 'rulebook/steering.md';
export const skillName = 'skills/verification-design/SKILL.md';
export const oracle =
	'docs/specs/2026-10-29-skill-rules-evidence-and-decisions/verification-design.md';

export function objectValue(value: JsonValue | undefined): JsonObject {
	requireCondition(
		value !== null && typeof value === 'object' && !Array.isArray(value),
		'Expected JSON object',
	);
	return value;
}
export function arrayValue(value: JsonValue | undefined): JsonValue[] {
	requireCondition(Array.isArray(value), 'Expected JSON array');
	return value;
}
const evidenceOrders = new WeakMap<JsonObject, string[]>();
/** JSON.parse sorts integer keys; read their lexical order before that information is lost. */
function evidenceOrder(raw: string): string[] {
	const tokens = [...raw.matchAll(/"(?:[^"\\]|\\[\s\S])*"|[{}[\]:,]/g)].map((match) => match[0]);
	let depth = 0;
	let reading = false;
	const keys: string[] = [];
	for (let index = 0; index < tokens.length; index++) {
		const token = tokens[index];
		if (
			depth === 1 &&
			token === '"evidence_seen"' &&
			tokens[index + 1] === ':' &&
			tokens[index + 2] === '{'
		)
			reading = true;
		else if (reading && depth === 2 && token?.startsWith('"') && tokens[index + 1] === ':') {
			const key: unknown = JSON.parse(token);
			keys.push(z.string().parse(key));
		}
		if (token === '{' || token === '[') depth++;
		if (token === '}' || token === ']') {
			depth--;
			if (reading && depth === 1) break;
		}
	}
	return keys;
}
export function parseObject(raw: string): JsonObject {
	const value: unknown = JSON.parse(raw);
	const result = jsonObjectSchema.parse(value);
	if ('evidence_seen' in result) evidenceOrders.set(result, evidenceOrder(raw));
	return result;
}
export function reportEvidenceEntries(report: JsonObject): [string, JsonValue][] {
	const order = evidenceOrders.get(report);
	requireCondition(
		order,
		'Compare a report parsed from its raw JSON to preserve emitted key order',
	);
	const states = objectValue(report['evidence_seen']);
	equal(order.length, Object.keys(states).length, 'Repeated or missing raw report keys');
	return order.map((key) => {
		const value = states[key];
		requireCondition(value !== undefined, 'Missing emitted request state');
		return [key, value];
	});
}
export function textValue(value: unknown): string {
	return z.string().parse(value);
}
export function numberValue(value: unknown): number {
	return z.number().parse(value);
}
export function readObject(file: string): JsonObject {
	return parseObject(filesystem.readFileSync(file, 'utf8'));
}
export function writeJson(file: string, value: unknown): void {
	filesystem.writeFileSync(file, `${JSON.stringify(jsonValueSchema.parse(value), null, 2)}\n`);
}
export function requireCondition(value: unknown, message: string): asserts value {
	assert.ok(value, message);
}
export function equal(actual: unknown, expected: unknown, message: string): void {
	assert.deepEqual(actual, expected, message);
}
export function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}
export function codePoints(value: string): number {
	return Array.from(value).length;
}
export function prepareFixture(destination: string, markers: readonly string[] = []): string {
	requireCondition(
		!filesystem.existsSync(destination),
		`Fixture destination exists: ${destination}`,
	);
	filesystem.cpSync(fixtureSource, destination, { recursive: true });
	if (markers.length > 0) {
		const rules = readObject(path.join(destination, rulesName));
		const selected: JsonObject = {};
		for (const marker of markers) {
			const entry = rules[marker];
			requireCondition(entry !== undefined, `Missing ${marker}`);
			selected[marker] = entry;
		}
		writeJson(path.join(destination, rulesName), selected);
		const lines = filesystem.readFileSync(path.join(destination, rulebookName), 'utf8').split('\n');
		filesystem.writeFileSync(
			path.join(destination, rulebookName),
			`${lines.filter((line) => markers.some((marker) => line.includes(`<!-- ${marker} -->`))).join('\n')}\n`,
		);
	}
	return destination;
}
export function configure(root: string, change: (config: JsonObject) => void): void {
	const config = readObject(path.join(root, configName));
	change(config);
	writeJson(path.join(root, configName), config);
}
export function selectWorkedSkill(root: string): void {
	configure(root, (config) => {
		config['check'] = { files: [skillName] };
	});
}
export function configureEndpoint(root: string, endpoint: string): void {
	configure(root, (config) => {
		config['decision_model'] = { ...objectValue(config['decision_model']), endpoint };
	});
}
export function gitCommand(root: string, arguments_: readonly string[]): string {
	const result = spawnSync(
		'git',
		['-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', ...arguments_],
		{
			cwd: root,
			encoding: 'utf8',
			timeout: 15000,
			env: {
				PATH: process.env['PATH'] ?? '/usr/bin:/bin',
				HOME: path.join(path.dirname(root), 'git-home'),
				GIT_CONFIG_NOSYSTEM: '1',
				GIT_CONFIG_GLOBAL: '/dev/null',
				GIT_AUTHOR_NAME: 'Rulebook fixture',
				GIT_AUTHOR_EMAIL: 'fixture@example.invalid',
				GIT_COMMITTER_NAME: 'Rulebook fixture',
				GIT_COMMITTER_EMAIL: 'fixture@example.invalid',
				GIT_AUTHOR_DATE: '2026-10-29T00:00:00Z',
				GIT_COMMITTER_DATE: '2026-10-29T00:00:00Z',
			},
		},
	);
	requireCondition(
		result.status === 0,
		`Fixture Git failed: ${result.stderr || result.error?.message}`,
	);
	return result.stdout.trim();
}
export function baselineGit(root: string): void {
	gitCommand(root, ['init', '-q']);
	gitCommand(root, ['add', '.']);
	gitCommand(root, ['commit', '-qm', 'Adopted rulebook fixture']);
}
export function oracleMatches(root: string): JsonObject[] {
	return arrayValue(readObject(path.join(root, 'expected/scan-oracle.json'))['matches']).map(
		objectValue,
	);
}
export function expectedSteps(root: string): JsonObject[] {
	return oracleMatches(root).filter(
		(item) => item['rule'] === 'step' && item['file'] === skillName,
	);
}
export function lintArguments(...extra: string[]): string[] {
	return ['lint', '--config', configName, '--format', 'json', ...extra];
}

const argumentsSchema = z.object({
	evidence: z.string(),
	prepare: z.boolean().default(false),
	binary: z.string().optional(),
	commit: z.string().optional(),
	'source-root': z.string().default(repository),
	sha256: z.string().optional(),
	case: z.array(z.string()).default([]),
});
export type JourneyArguments = z.infer<typeof argumentsSchema>;
export function journeyArguments(description: string): JourneyArguments | undefined {
	const parsed = parseArgs({
		options: {
			evidence: { type: 'string' },
			prepare: { type: 'boolean' },
			binary: { type: 'string' },
			commit: { type: 'string' },
			'source-root': { type: 'string' },
			sha256: { type: 'string' },
			case: { type: 'string', multiple: true },
			help: { type: 'boolean' },
		},
	});
	if (parsed.values.help) {
		process.stdout.write(
			`${description}\n--prepare --evidence <new-directory> writes fixtures without a product.\nObservation: --binary <absolute> --source-root <checkout> --commit <full-head> [--sha256 <declared>] --evidence <new-directory>.\n--case <name> selects a supported bounded S1 rerun. No hosted calls or credential access.\n`,
		);
		return undefined;
	}
	const values = argumentsSchema.parse(parsed.values);
	requireCondition(
		values.prepare || (values.binary && values.commit),
		'Observation requires --binary and full --commit',
	);
	return values;
}

/** One owned child at a time: captures and cancellation belong to that child. */
export async function runSequence<Value>(
	values: readonly Value[],
	visit: (value: Value) => Promise<void>,
): Promise<void> {
	await values.reduce<Promise<void>>(
		(previous, value) => previous.then(() => visit(value)),
		Promise.resolve(),
	);
}
