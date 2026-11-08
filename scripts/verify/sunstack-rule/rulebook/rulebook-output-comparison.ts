/** Human pointers and exact output preservation, with an explicit engine-version exception. */
import {
	arrayValue,
	equal,
	objectValue,
	parseObject,
	requireCondition,
} from './rulebook-fixture-support.ts';

export interface SummaryEntry {
	readonly id: number;
	readonly file: string;
	readonly line: number;
	readonly kind: 'warn' | 'unsure' | 'skip' | 'incomplete';
	readonly rule?: string;
}

function escapedPattern(text: string): string {
	return text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/** Findings remain located; every unsure group has its count and a retained result pointer. */
export function requireHumanSummary(output: string, entries: readonly SummaryEntry[]): void {
	requireCondition(
		!/BODY_SENTINEL|CONTEXT_ONLY_SENTINEL|HISTORICAL_EXTERNAL_OPENING_SENTINEL/.test(output),
		'Human output leaked request text',
	);
	const lines = output.split('\n');
	requireCondition(
		!lines.some((line) => line.includes(': unsure [')),
		'Individual unsure rows remain',
	);
	const unsure = new Map<string, number[]>();
	for (const entry of entries) {
		if (entry.kind === 'unsure') {
			requireCondition(entry.rule, 'Unsure expectation needs its governing rule');
			const ids = unsure.get(entry.rule) ?? [];
			ids.push(entry.id);
			unsure.set(entry.rule, ids);
			continue;
		}
		const selected = lines.filter((line) => new RegExp(`#${entry.id}(?![0-9])`).test(line));
		equal(selected.length, 1, 'Summary ID missing or duplicated');
		const line = selected[0];
		requireCondition(line, 'Summary line absent');
		const locationIndex = line.indexOf(entry.file);
		const actualLine = line.slice(locationIndex + entry.file.length).match(/^:([0-9]+)/);
		requireCondition(locationIndex >= 0 && actualLine, 'Summary lacks file:line');
		equal(Number(actualLine[1]), entry.line, 'Summary points at the wrong line');
		requireCondition(
			new RegExp(`\\b${entry.kind === 'skip' ? 'skip(?:ped)?' : entry.kind}\\b`, 'i').test(line),
			'Summary kind differs',
		);
	}
	const summaries = lines.filter((line) => /^\d+ unsure \[/.test(line));
	equal(summaries.length, unsure.size, 'Unsure rule summary missing or duplicated');
	for (const [rule, ids] of unsure) {
		const selected = summaries.filter((line) => line.includes(`unsure [${rule}]`));
		equal(selected.length, 1, 'Unsure rule summary missing or duplicated');
		const match = selected[0]?.match(
			new RegExp(
				`^(\\d+) unsure \\[${escapedPattern(rule)}\\].*lint show <report> (\\d+)(?:\\s|$)`,
			),
		);
		requireCondition(match, 'Unsure summary lacks count or detail pointer');
		equal(Number(match[1]), ids.length, 'Unsure count differs');
		requireCondition(ids.includes(Number(match[2])), 'Unsure pointer names another result');
	}
}

export type ComparedFormat = 'json' | 'report' | 'github' | 'sarif' | 'show';

/** Equal versions are always strict. Different versions permit only the identified engine field. */
export function requireComparableBytes(options: {
	before: Buffer;
	after: Buffer;
	format: ComparedFormat;
	baselineVersion: string;
	candidateVersion: string;
}): void {
	if (['json', 'report', 'sarif'].includes(options.format)) {
		const engineVersion = (report: ReturnType<typeof parseObject>): unknown =>
			options.format === 'sarif'
				? objectValue(objectValue(objectValue(arrayValue(report['runs'])[0])['tool'])['driver'])[
						'version'
					]
				: report['version'];
		equal(
			engineVersion(parseObject(options.before.toString('utf8'))),
			options.baselineVersion,
			'Baseline engine version differs',
		);
		equal(
			engineVersion(parseObject(options.after.toString('utf8'))),
			options.candidateVersion,
			'Candidate engine version differs',
		);
	}
	if (options.before.equals(options.after)) return;
	requireCondition(
		options.baselineVersion !== options.candidateVersion,
		'Same-version bytes changed',
	);
	requireCondition(
		['json', 'report', 'sarif'].includes(options.format),
		'Versionless output changed',
	);
	const pattern = new RegExp(
		`^(\\s*"version": )${escapedPattern(JSON.stringify(options.baselineVersion))}(,?)$`,
		'gm',
	);
	const raw = options.before.toString('utf8');
	equal([...raw.matchAll(pattern)].length, 1, 'Engine-version exception is not unique');
	const adjusted = raw.replace(
		pattern,
		(_match: string, prefix: string, suffix: string) =>
			`${prefix}${JSON.stringify(options.candidateVersion)}${suffix}`,
	);
	equal(Buffer.from(adjusted), options.after, 'Bytes outside the engine-version field changed');
}

export function requireHumanParity(before: Buffer, after: Buffer): void {
	const oldLines = before.toString('utf8').split('\n');
	const newLines = after.toString('utf8').split('\n');
	equal(
		oldLines.filter((line) => !line.includes(': unsure [')),
		newLines.filter((line) => !/^\d+ unsure \[/.test(line)),
		'Human output changed outside unsure summaries',
	);
}
