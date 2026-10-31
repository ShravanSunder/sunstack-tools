/** Caller observations compared with independent S0 source and scan values. */
import filesystem from 'node:fs';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { z } from 'zod';

import {
	arrayValue,
	codePoints,
	configName,
	equal,
	jsonObjectSchema,
	numberValue,
	objectValue,
	oracleMatches,
	parseObject,
	reportEvidenceEntries,
	readObject,
	requireCondition,
	skillName,
	textValue,
	type JsonObject,
} from './rulebook-fixture-support.ts';

const listingSchema = z.object({
	version: z.string(),
	config: z.string(),
	scope: jsonObjectSchema,
	pieces: z.array(jsonObjectSchema),
	usage: jsonObjectSchema,
});
export function listingPieces(raw: Buffer): JsonObject[] {
	return listingSchema.parse(JSON.parse(raw.toString('utf8'))).pieces;
}
export function location(piece: JsonObject): JsonObject {
	return objectValue(piece['where']);
}
function signature(piece: JsonObject): [string, string, number, number] {
	const where = location(piece);
	const span = objectValue(where['span']);
	return [
		textValue(piece['kind']),
		textValue(where['file']),
		numberValue(span['byte_start']),
		numberValue(span['byte_end']),
	];
}
export function requireRequestPiece(options: {
	state: JsonObject;
	piece: JsonObject;
	standard: string;
	questions: JsonObject;
}): void {
	const view = objectValue(options.piece['model_view']);
	equal(view['state'], 'bounded', 'TooLarge must not dispatch');
	const unavailable = new Set(
		Object.entries(objectValue(options.piece['fields']))
			.filter(([_name, value]) => objectValue(value)['state'] === 'unavailable')
			.map(([name]) => name),
	);
	const references = new Set<string>();
	for (const question of Object.values(options.questions)) {
		const instructions = objectValue(question)['instructions'];
		if (instructions && typeof instructions === 'object' && !Array.isArray(instructions))
			for (const key of ['check', 'against', 'context']) {
				const names = instructions[key];
				if (typeof names === 'string') references.add(names);
				else if (Array.isArray(names))
					for (const name of names) if (typeof name === 'string') references.add(name);
			}
	}
	requireCondition(
		![...references].some((name) => unavailable.has(name)),
		'An explicitly referenced unavailable field must block dispatch',
	);
	const fields = Object.fromEntries(
		Object.entries(objectValue(view['fields'])).filter(([name]) => !unavailable.has(name)),
	);
	equal(
		options.state,
		{ standard: options.standard, where: options.piece['where'], ...fields },
		'Request state differs from exactly one piece model view',
	);
}
function multiset(bodies: Buffer[]): [string, number][] {
	const counts = new Map<string, number>();
	for (const body of bodies) {
		const raw = body.toString('base64');
		counts.set(raw, (counts.get(raw) ?? 0) + 1);
	}
	return [...counts].toSorted(([left], [right]) => left.localeCompare(right));
}
function logicalBodies(report: JsonObject, bodies: Buffer[]): [string, string][] {
	const records = reportEvidenceEntries(report);
	equal(records.length, bodies.length, 'Request evidence count differs from captures');
	const captures = bodies.map((raw) => ({
		raw: raw.toString('base64'),
		state: parseObject(raw.toString('utf8'))['state'],
	}));
	return records.map(([index, state]) => {
		const matching = captures.filter((capture) => isDeepStrictEqual(capture.state, state));
		const found = matching[0];
		requireCondition(found && matching.length === 1, 'Report record has no unique captured body');
		return [index, found.raw];
	});
}
export function requireRepeatedRequests(options: {
	first: JsonObject;
	second: JsonObject;
	firstBodies: Buffer[];
	secondBodies: Buffer[];
}): void {
	equal(
		multiset(options.firstBodies),
		multiset(options.secondBodies),
		'Arrival-side request bodies changed',
	);

	// Only arrival-side data is sorted; emitted report order remains observable.
	equal(
		logicalBodies(options.first, options.firstBodies),
		logicalBodies(options.second, options.secondBodies),
		'Report request order or exact body bytes changed',
	);
}
export function requirePieceContract(pieces: JsonObject[], root: string): void {
	const kinds = objectValue(
		objectValue(readObject(path.join(root, configName))['evidence'])['kinds'],
	);
	const matches = oracleMatches(root);
	const expected: [string, string, number, number][] = [];
	for (const [kind, value] of Object.entries(kinds))
		for (const item of matches)
			if (
				item['rule'] === objectValue(value)['rule'] &&
				textValue(item['file']).endsWith('/SKILL.md')
			)
				expected.push([
					kind,
					textValue(item['file']),
					numberValue(item['start']),
					numberValue(item['end']),
				]);
	expected.sort(
		(left, right) =>
			left[0].localeCompare(right[0]) || left[1].localeCompare(right[1]) || left[2] - right[2],
	);
	equal(
		pieces.map(signature),
		expected,
		'Pieces differ from independent scan oracle or contracted order',
	);
	const selected = pieces.filter((piece) =>
		isDeepStrictEqual(signature(piece), ['step', skillName, 6114, 7909]),
	);
	const piece = selected[0];
	requireCondition(piece && selected.length === 1, 'Step 3 was split or lost');
	equal(
		[location(piece)['first_line'], location(piece)['last_line']],
		[49, 60],
		'Step 3 line range differs',
	);
	const fields = objectValue(piece['fields']);
	const heading = objectValue(fields['heading']);
	equal(heading['state'], 'text', 'Heading state differs');
	equal(heading['value'], '3. Choose each observation', 'Heading differs');
	const completion = objectValue(fields['completion']);
	equal(completion['state'], 'texts', 'Completion lost list state');
	const spans = arrayValue(completion['spans']);
	const firstSpan = spans[0];
	requireCondition(
		firstSpan && spans.length === 1 && objectValue(firstSpan)['start_line'] === 59,
		'Completion attached to another step',
	);
	equal(
		completion['values'],
		matches
			.filter(
				(item) =>
					item['rule'] === 'completion' &&
					item['file'] === skillName &&
					numberValue(item['start']) >= 6114 &&
					numberValue(item['end']) <= 7909,
			)
			.map((item) => item['text']),
		'Completion text differs from independent scan',
	);
	const chunks =
		filesystem.readFileSync(path.join(root, skillName), 'utf8').match(/[^\n]*\n|[^\n]+$/g) ?? [];
	const body = objectValue(fields['body']);
	equal(body['state'], 'text', 'Body state differs');
	equal(
		body['value'],
		[...chunks.slice(49, 58), ...chunks.slice(59, 60)].join(''),
		'Body grouping differs',
	);
	const view = objectValue(piece['model_view']);
	equal(view['state'], 'bounded', 'Small piece must be bounded');
	equal(view['cut'], [], 'Small piece unexpectedly cut');
	equal(
		view['fields'],
		{ heading: heading['value'], completion: completion['values'], body: body['value'] },
		'Small model view differs from independently checked raw fields',
	);
	const loaded = pieces.filter((item) =>
		isDeepStrictEqual(signature(item), ['load', skillName, 6146, 6445]),
	);
	const load = loaded[0];
	requireCondition(load && loaded.length === 1, 'Line 51 load missing');
	const targets = arrayValue(objectValue(objectValue(load['fields'])['targets'])['values']).map(
		objectValue,
	);
	const target = targets.find((item) => item['name'] === 'references/choosing-the-observation.md');
	requireCondition(target && target['exists'] === true, 'Load target did not resolve');
	equal(
		target['opening'],
		Array.from(
			filesystem.readFileSync(
				path.join(root, 'skills/verification-design/references/choosing-the-observation.md'),
				'utf8',
			),
		)
			.slice(0, 3000)
			.join(''),
		'Target opening differs',
	);
	const retrospective = pieces.find(
		(item) =>
			item['kind'] === 'load' &&
			location(item)['file'] === 'skills/practices-retrospective/SKILL.md' &&
			location(item)['first_line'] === 21,
	);
	requireCondition(retrospective, 'Retrospective load absent');
	const research = arrayValue(
		objectValue(objectValue(retrospective['fields'])['targets'])['values'],
	)
		.map(objectValue)
		.find((item) => item['name'] === 'practices-research');
	requireCondition(research && research['exists'] === true, 'Named skill did not resolve');
	const nested = objectValue(research['fields']);
	equal(objectValue(nested['description'])['state'], 'text', 'Named skill fields absent');
	requireCondition(!('targets' in nested), 'Target composition became recursive');
	const skill = pieces.find(
		(item) => item['kind'] === 'skill' && location(item)['file'] === skillName,
	);
	requireCondition(skill, 'Skill piece absent');
	const description = objectValue(objectValue(skill['fields'])['description']);
	equal(description['state'], 'text', 'Description state differs');
	equal(codePoints(textValue(description['value'])), 450, 'Description YAML value differs');
}
