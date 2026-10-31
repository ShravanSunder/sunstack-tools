/** U10 compares every legacy value and emitted key/array order, allowing additive fields only. */
import { equal, jsonValueSchema, requireCondition } from './rulebook-fixture-support.ts';

type OrderedValue =
	| { kind: 'object'; entries: [string, OrderedValue][] }
	| { kind: 'array'; values: OrderedValue[] }
	| { kind: 'scalar'; value: string | number | boolean | null };

function orderedJson(raw: string): OrderedValue {
	// Validate JSON first. Retain key order before JS reorders integer property names.
	jsonValueSchema.parse(JSON.parse(raw));
	const tokens = [
		...raw.matchAll(
			/"(?:[^"\\]|\\[\s\S])*"|-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?|true|false|null|[{}[\]:,]/g,
		),
	].map((match) => match[0]);
	let position = 0;
	function readValue(): OrderedValue {
		const token = tokens[position++];
		requireCondition(token !== undefined, 'Unexpected end of ordered JSON');
		if (token === '{') {
			const entries: [string, OrderedValue][] = [];
			while (tokens[position] !== '}') {
				const key = jsonValueSchema.parse(JSON.parse(tokens[position++] ?? 'null'));
				requireCondition(
					typeof key === 'string' && tokens[position++] === ':',
					'Malformed ordered key',
				);
				requireCondition(!entries.some(([prior]) => prior === key), 'Duplicate report key');
				entries.push([key, readValue()]);
				if (tokens[position] !== ',') break;
				position++;
			}
			equal(tokens[position++], '}', 'Unclosed ordered object');
			return { kind: 'object', entries };
		}
		if (token === '[') {
			const values: OrderedValue[] = [];
			while (tokens[position] !== ']') {
				values.push(readValue());
				if (tokens[position] !== ',') break;
				position++;
			}
			equal(tokens[position++], ']', 'Unclosed ordered array');
			return { kind: 'array', values };
		}
		const value = jsonValueSchema.parse(JSON.parse(token));
		requireCondition(value === null || typeof value !== 'object', 'Expected scalar JSON');
		return { kind: 'scalar', value };
	}
	const value = readValue();
	equal(position, tokens.length, 'Unconsumed ordered JSON');
	return value;
}
function compareLegacy(before: OrderedValue, after: OrderedValue, location: string): void {
	requireCondition(before.kind === after.kind, `${location}: legacy type changed`);
	if (before.kind === 'object' && after.kind === 'object') {
		const baselineKeys = before.entries.map(([key]) => key);
		const oldKeys = new Set(baselineKeys);
		equal(
			after.entries.filter(([key]) => oldKeys.has(key)).map(([key]) => key),
			baselineKeys,
			`${location}: legacy key membership/order changed`,
		);
		for (const [key, value] of before.entries) {
			const candidate = after.entries.find(([name]) => name === key);
			requireCondition(candidate, `${location}.${key}: legacy key missing`);
			if (location === '$' && key === 'version') {
				equal(
					candidate[1],
					{ kind: 'scalar', value: '0.6.0' },
					'U10 requires report version 0.6.0',
				);
			} else compareLegacy(value, candidate[1], `${location}.${key}`);
		}
	} else if (before.kind === 'array' && after.kind === 'array') {
		equal(
			after.values.length,
			before.values.length,
			`${location}: legacy array membership changed`,
		);
		before.values.forEach((value, index) => {
			const candidate = after.values[index];
			requireCondition(candidate, `${location}[${index}]: legacy item missing`);
			compareLegacy(value, candidate, `${location}[${index}]`);
		});
	} else if (before.kind === 'scalar' && after.kind === 'scalar')
		equal(after.value, before.value, `${location}: legacy value changed`);
}
export function requireNudgeCompatibility(baseline: string, current: string): void {
	const before = orderedJson(baseline);
	const after = orderedJson(current);
	requireCondition(
		before.kind === 'object' && before.entries.some(([key]) => key === 'version'),
		'Baseline report version absent',
	);
	equal(
		before.entries.find(([key]) => key === 'version')?.[1],
		{ kind: 'scalar', value: '0.5.0' },
		'Baseline must be 0.5.0',
	);
	compareLegacy(before, after, '$');
}
