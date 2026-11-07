/** O5 report fixture: five observable outcomes, scripted backend, no real credentials. */
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { isDeepStrictEqual } from 'node:util';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	excludedFolder,
	filterQuestions,
	filterReply,
	includedFolder,
	includedMembers,
	prepareFilterFixture,
} from './rulebook-filters.ts';
import {
	arrayValue,
	configName,
	configureEndpoint,
	equal,
	configure,
	journeyArguments,
	objectValue,
	numberValue,
	parseObject,
	readObject,
	requireCondition,
	rulebookName,
	rulesName,
	runSequence,
	textValue,
	writeJson,
	type JsonObject,
	type JourneyArguments,
} from './rulebook-fixture-support.ts';
import {
	declaredTarget,
	responseBody,
	RulebookProcess,
	type ControlledReply,
	type CapturedRequest,
} from './rulebook-process.ts';

export const reportOracle =
	'aa2f850:docs/specs/2026-11-06-decision-trees-everywhere/verification-design.html#o5';
export const skippedReportFolder = 'plugins/three/skills/review';
export const skippedDescription = 'Uncertain report guidance.';
export const gapReportFile = `${includedFolder}/notes/four.md`;
export const reportRuleReference: JsonObject = {
	examples: [
		{
			text: 'Approved instruction.',
			expected: 'pass',
			why: 'Author-provided reference, not a case label.',
		},
	],
};

/** Fixture values select pass/warn/unsure/incomplete; a third folder supplies a filter skip. */
export function prepareReportFixture(destination: string): string {
	prepareFilterFixture(destination);
	const bodies = [
		'Approved PASS_BODY_SENTINEL.',
		'Approved WARN_BODY_SENTINEL.',
		'Approved UNSURE_BODY_SENTINEL.',
	];
	for (const [index, member] of includedMembers.entries())
		filesystem.writeFileSync(path.join(destination, member), `${bodies[index]}\n`);
	filesystem.writeFileSync(path.join(destination, gapReportFile), 'Approved GAP_BODY_SENTINEL.\n');
	filesystem.mkdirSync(path.join(destination, skippedReportFolder, 'notes'), { recursive: true });
	filesystem.writeFileSync(
		path.join(destination, skippedReportFolder, 'overview.md'),
		`---\ndescription: ${skippedDescription}\nopening: Unsure opening.\n---\n`,
	);
	filesystem.writeFileSync(
		path.join(destination, skippedReportFolder, 'notes/one.md'),
		'# Skipped fixture\n\n\n\nApproved SKIP_BODY_SENTINEL.\n',
	);
	const rules = readObject(path.join(destination, rulesName));
	const asking = objectValue(rules['K002']);
	asking['reference'] = reportRuleReference;
	asking['questions'] = {
		verdict: {
			type: 'noul',
			instructions: {
				check: 'body',
				against: 'reference.examples',
				task: 'Is this instruction approved?',
			},
		},
	};
	asking['decision'] = {
		start: 'approved',
		steps: {
			approved: {
				ask: 'verdict',
				yes_at_or_above: 0.7,
				no_at_or_below: 0.3,
				yes: 'pass',
				['no']: { finding: 'Read the instruction and decide whether approval is missing.' },
				uncertain: 'uncertain',
			},
		},
	};
	writeJson(path.join(destination, rulesName), rules);
	configure(destination, (config) => {
		config['decision_model'] = { model: 'typesafe/requested-report' };
	});
	return destination;
}
export function reportReply(body: JsonObject): ControlledReply {
	const state = objectValue(body['state']);
	if ('scope' in objectValue(body['questions'])) {
		if (state['description'] === skippedDescription) {
			const reply = responseBody(body);
			objectValue(reply['answers'])['scope'] = {
				type: 'choice',
				choice: 'reusable',
				confidence: 0.99,
				probabilities: { reusable: 0.5, product: 0.25, other: 0.25 },
			};
			return { status: 200, body: reply };
		}
		return { status: 200, body: filterReply(body, 'judged') };
	}
	const text = state['body'];
	requireCondition(typeof text === 'string', 'Report rule request lacks body');
	if (text.includes('GAP_BODY_SENTINEL'))
		return { status: 400, body: { error: 'controlled report refusal' } };
	const probability = text.includes('WARN_BODY_SENTINEL')
		? 0.1
		: text.includes('UNSURE_BODY_SENTINEL')
			? 0.5
			: 0.95;
	return { status: 200, body: responseBody(body, probability) };
}
export function requireReportRequestShape(body: JsonObject): void {
	const state = objectValue(body['state']);
	if ('scope' in objectValue(body['questions'])) {
		requireCondition(!('where' in state), 'Filter location leaked');
		equal(body['questions'], filterQuestions, 'Filter questions changed');
	} else {
		requireCondition(!('where' in state), 'Unasked rule location leaked');
		equal(state['reference'], reportRuleReference, 'Reference changed');
	}
	requireCondition(!JSON.stringify(state).includes(excludedFolder), 'Folder path leaked');
}

/** Join a saved request to independently captured HTTP and the controlled response. */
export function requireSavedRequest(options: {
	sent: JsonObject;
	savedState: unknown;
	savedQuestions: unknown;
	savedAnswers: unknown;
	requestedModel: unknown;
	resolvedModel: unknown;
	reply: ControlledReply;
}): void {
	equal(options.savedState, options.sent['state'], 'Saved state differs from sent state');
	equal(
		options.savedQuestions,
		options.sent['questions'],
		'Saved questions differ from sent questions',
	);
	equal(
		options.requestedModel,
		options.sent['model'],
		'Requested model differs from actual request',
	);
	if (options.reply.status !== 200) {
		equal(options.resolvedModel, null, 'Failed request invented a responding model');
		equal(options.savedAnswers, undefined, 'Failed request invented answers');
		return;
	}
	equal(
		options.resolvedModel,
		options.reply.body['model'],
		'Responding model differs from actual reply',
	);
	const expected: JsonObject = {};
	for (const [name, value] of Object.entries(objectValue(options.reply.body['answers']))) {
		const answer = objectValue(value);
		if (answer['type'] === 'noul')
			expected[name] = { type: 'noul', yes_probability: answer['noul'] ?? null };
		else if (answer['type'] === 'choice')
			expected[name] = {
				type: 'choice',
				selected_label: answer['choice'] ?? null,
				confidence: answer['confidence'] ?? null,
				probabilities: answer['probabilities'] ?? null,
			};
		else throw new Error('Report fixture has an unsupported controlled answer primitive');
	}
	equal(
		options.savedAnswers,
		expected,
		'Saved report lost or changed returned answers, including unused ones',
	);
}
export interface SummaryEntry {
	readonly id: number;
	readonly file: string;
	readonly line: number;
	readonly kind: 'warn' | 'unsure' | 'skip' | 'incomplete';
}
/** Each known entry must be brief, located and addressable; fixture evidence must stay out. */
export function requireHumanSummary(output: string, entries: readonly SummaryEntry[]): void {
	requireCondition(
		!/BODY_SENTINEL|CONTEXT_ONLY_SENTINEL|HISTORICAL_EXTERNAL_OPENING_SENTINEL/.test(output),
		'Human output leaked request text',
	);
	const lines = output.split('\n');
	for (const entry of entries) {
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
}

/** Saved request/model/state evidence is joined to actual captured requests, never self-reported counts. */
export function requireReportCaptures(
	report: JsonObject,
	captures: readonly CapturedRequest[],
): void {
	const requests = arrayValue(report['requests']).map(objectValue);
	equal(captures.length, 7, 'Report fixture did not drive all filter/rule requests');
	equal(requests.length, captures.length, 'Saved request missing, duplicated or invented');
	const seen = new Set<number>();
	for (const record of requests) {
		const request = objectValue(record['request']);
		const matches = captures
			.map((capture, index) => ({ capture, index }))
			.filter(
				({ capture }) =>
					isDeepStrictEqual(capture.body['state'], request['state']) &&
					isDeepStrictEqual(capture.body['questions'], request['questions']) &&
					capture.body['model'] === record['requested_model'],
			);
		equal(matches.length, 1, 'Saved request does not identify exactly one real request');
		const matched = matches[0];
		requireCondition(matched, 'Captured request missing');
		requireCondition(!seen.has(matched.index), 'Two saved records identify the same request');
		seen.add(matched.index);
		requireReportRequestShape(matched.capture.body);
		equal(record['state'], 'run', 'Dispatched request marked not sent');
		const reply = reportReply(matched.capture.body);
		const outcome = objectValue(record['outcome']);
		if (reply.status === 400) {
			equal(outcome['failure_class'], 'provider_rejected_request', 'Saved refusal class differs');
			equal(outcome['http_status'], 400, 'Saved refusal status differs');
		}
		equal(
			outcome['kind'],
			reply.status === 200 ? 'completed' : 'provider_failure',
			'Saved outcome differs from controlled response',
		);
		requireSavedRequest({
			sent: matched.capture.body,
			savedState: request['state'],
			savedQuestions: request['questions'],
			savedAnswers: outcome['answers'],
			requestedModel: record['requested_model'],
			resolvedModel: record['resolved_model'],
			reply,
		});
		const owner = objectValue(record['owner']);
		if ('scope' in objectValue(request['questions'])) {
			equal(owner['kind'], 'filter', 'Wrong request owner kind');
			equal(owner['filter'], 'category', 'Filter owner lost');
			const description = objectValue(request['state'])['description'];
			equal(
				owner['folder'],
				description === skippedDescription
					? skippedReportFolder
					: description === 'General instructions for an included folder.'
						? includedFolder
						: excludedFolder,
				'Filter request joined to wrong folder',
			);
		} else {
			equal(owner['kind'], 'rule', 'Wrong request owner kind');
			equal(owner['rule'], 'asking_gate', 'Rule request owner lost');
			const body = textValue(objectValue(request['state'])['body']);
			const file = body.includes('PASS_BODY')
				? includedMembers[0]
				: body.includes('WARN_BODY')
					? includedMembers[1]
					: body.includes('UNSURE_BODY')
						? includedMembers[2]
						: gapReportFile;
			equal(objectValue(owner['piece'])['file'], file, 'Rule request joined to wrong piece');
		}
	}
}
function requireRulebookSnapshots(report: JsonObject, root: string): void {
	const required = [configName, rulebookName, rulesName].map((file) => path.resolve(root, file));
	const actual: string[] = [];
	for (const file of arrayValue(report['rulebook_files']).map(objectValue)) {
		const savedPath = path.resolve(root, textValue(file['path']));
		requireCondition(savedPath.startsWith(`${root}/`), 'Snapshot points outside isolated fixture');
		const original = filesystem.readFileSync(savedPath, 'utf8');
		equal(file['content'], original, 'Saved rulebook content differs from original');
		equal(
			file['sha256'],
			createHash('sha256').update(original).digest('hex'),
			'Saved rulebook content hash differs',
		);
		actual.push(savedPath);
	}
	for (const file of required)
		requireCondition(actual.includes(file), 'Saved report omitted config/Markdown/rules');
	equal(new Set(actual).size, actual.length, 'Rulebook file snapshot duplicated');
}
function selectedReportEntries(report: JsonObject): JsonObject[] {
	const results = arrayValue(report['results']).map(objectValue);
	const asking = results.filter((row) => row['name'] === 'asking_gate');
	for (const [index, outcome] of ['pass', 'finding', 'uncertain', 'incomplete'].entries()) {
		const file = index === 3 ? gapReportFile : includedMembers[index];
		const rows = asking.filter((row) => row['file'] === file);
		equal(rows.length, 1, 'Report missing or duplicated required rule result');
		equal(rows[0]?.['outcome'], outcome, 'Scripted result has wrong outcome');
		equal(rows[0]?.['path'], ['approved'], 'Rule path lost');
	}
	const excluded = asking.find((row) => row['outcome'] === 'not_applicable');
	requireCondition(excluded, 'Excluded rule result missing');
	const skipped = arrayValue(report['filter_skips']).map(objectValue);
	equal(skipped.length, 2, 'Filter skips missing or duplicated');
	for (const skip of skipped) {
		equal(
			arrayValue(skip['locations'])
				.map(objectValue)
				.map((location) => ({
					file: location['file'],
					first_line: location['first_line'],
					last_line: location['last_line'],
				})),
			[{ file: `${skippedReportFolder}/notes/one.md`, first_line: 5, last_line: 5 }],
			'Filter skip lost its actual non-first-line piece',
		);
	}
	equal(
		skipped.map((row) => row['file']),
		[`${skippedReportFolder}/notes/one.md`, `${skippedReportFolder}/notes/one.md`],
		'Wrong skipped folder',
	);
	const chosen = ['pass', 'finding', 'uncertain', 'incomplete'].map((outcome) => {
		const row = asking.find((item) => item['outcome'] === outcome);
		requireCondition(row, 'Missing result kind');
		return row;
	});
	return [...chosen, ...skipped, excluded];
}
function requireSingleDetail(detail: JsonObject, row: JsonObject, report: JsonObject): void {
	equal(detail['result'], row, 'Show selected the wrong result or changed it');
	requireCondition(
		!('results' in detail) && !('filter_answers' in detail) && !('filter_skips' in detail),
		'Show returned the whole report',
	);
	const ids = arrayValue(row['request_ids']);
	const requests = arrayValue(report['requests'])
		.map(objectValue)
		.filter((record) => ids.includes(record['id'] ?? null));
	equal(detail['requests'], requests, 'Show lost or included unrelated requests');
	equal(
		detail['rulebook_files'],
		report['rulebook_files'],
		'Show cannot explain tree/bars from saved files',
	);
	if (row['outcome'] === 'not_applicable' || row['why']) {
		equal(detail['rewalkable'], false, 'Excluded/skipped rule became re-walkable');
		requireCondition(
			typeof detail['gap'] === 'string' && detail['gap'].includes('never asked'),
			'Excluded/skipped rule lacks explanation',
		);
	}
	if (row['outcome'] === 'incomplete')
		requireCondition(
			row['gap'] !== null && row['gap'] !== undefined,
			'Incomplete result lost its gap',
		);
}

export async function runReportJourney(options: JourneyArguments): Promise<void> {
	const target = options.prepare
		? undefined
		: declaredTarget({
				binary: options.binary ?? '',
				source: options['source-root'],
				commit: options.commit ?? '',
				...(options.sha256 ? { digest: options.sha256 } : {}),
			});
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const root = prepareReportFixture(path.join(evidence, 'fixture'));
	if (options.prepare) {
		writeJson(path.join(evidence, 'prepared.json'), {
			product_observed: false,
			oracle: reportOracle,
			config: configName,
			outcomes: ['pass', 'warn', 'unsure', 'filter_skip', 'incomplete'],
		});
		process.stdout.write('Prepared report fixtures; product unobserved\n');
		return;
	}
	requireCondition(target, 'Report observation needs declared candidate');
	requireDebugSignature(target.binary);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		configureEndpoint(root, observer.endpoint);
		observer.respond = ({ body }) => Promise.resolve(reportReply(body));
		const check = await observer.run({
			root,
			name: 'check',
			arguments: ['lint', '--config', configName, '--check'],
			expectedExit: 0,
		});
		equal(check.captures.length, 0, 'Keyless check dispatched');
		const saved = path.join(evidence, 'saved-report.json');
		const run = await observer.run({
			root,
			name: 'human',
			arguments: ['lint', '--config', configName, '--all', '--report', saved],
			credential: true,
			expectedExit: 3,
		});
		const report = readObject(saved);
		equal(objectValue(report['status'])['kind'], 'incomplete', 'Gap did not dominate run status');
		requireReportCaptures(report, run.captures);
		requireRulebookSnapshots(report, root);
		const entries = selectedReportEntries(report);
		const ids = [
			...arrayValue(report['results']),
			...arrayValue(report['filter_answers']),
			...arrayValue(report['filter_skips']),
		]
			.map(objectValue)
			.map((row) => numberValue(row['id']));
		equal(new Set(ids).size, ids.length, 'Result IDs collide across result kinds');
		const summaries: SummaryEntry[] = entries
			.filter((row) => row['outcome'] !== 'pass' && row['outcome'] !== 'not_applicable')
			.map((row) => ({
				id: numberValue(row['id']),
				file: textValue(row['file']),
				line: row['why'] ? 5 : 1,
				kind: row['why']
					? 'skip'
					: row['outcome'] === 'finding'
						? 'warn'
						: row['outcome'] === 'uncertain'
							? 'unsure'
							: 'incomplete',
			}));
		requireHumanSummary(
			`${run.stdout.toString('utf8')}\n${run.stderr.toString('utf8')}`,
			summaries,
		);
		requireCondition(
			/\bread\b/i.test(run.stdout.toString('utf8')) &&
				/\bdecide\b/i.test(run.stdout.toString('utf8')),
			'Warning is not phrased as a nudge',
		);
		const details = new Map<number, JsonObject>();
		await runSequence(entries, async (row) => {
			const id = numberValue(row['id']);
			const result = await observer.run({
				root,
				name: `show-${id}`,
				arguments: ['lint', 'show', saved, String(id)],
				expectedExit: 0,
			});
			equal(result.captures.length, 0, 'Show dispatched');
			const detail = parseObject(result.stdout.toString('utf8'));
			requireSingleDetail(detail, row, report);
			details.set(id, detail);
		});
		const invalidId = Math.max(...ids) + 100;
		const invalid = await observer.run({
			root,
			name: 'show-invalid',
			arguments: ['lint', 'show', saved, String(invalidId)],
			expectedExit: 2,
		});
		const error = invalid.stderr.toString('utf8');
		requireCondition(
			error.includes(String(invalidId)) &&
				new RegExp(
					`(?:range|valid[^\\n]*ids?)[^\\n]*\\b${Math.min(...ids)}\\b[^\\n]*\\b${Math.max(...ids)}\\b`,
					'i',
				).test(error),
			'Unknown-ID error lacks ID and valid range',
		);
		filesystem.writeFileSync(path.join(root, configName), 'changed rulebook configuration\n');
		filesystem.unlinkSync(path.join(root, rulebookName));
		filesystem.unlinkSync(path.join(root, rulesName));
		await runSequence(entries, async (row) => {
			const id = numberValue(row['id']);
			const result = await observer.run({
				root,
				name: `show-after-delete-${id}`,
				arguments: ['lint', 'show', saved, String(id)],
				expectedExit: 0,
			});
			equal(result.captures.length, 0, 'Show re-ran after rulebook deletion');
			const detail = parseObject(result.stdout.toString('utf8'));
			requireSingleDetail(detail, row, report);
			equal(detail, details.get(id), 'Saved explanation depends on changed/deleted rulebook');
		});
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		product_observed: true,
		oracle: reportOracle,
		commit: target.commit,
		binary_sha256: target.digest,
		outcomes: ['pass', 'warn', 'unsure', 'filter_skip', 'incomplete'],
		unverified: ['keyed ai-tools run', 'judgment quality', 'release publication'],
	});
	process.stdout.write('Saved report and one-entry detail journey passed\n');
}
if (process.argv[1] === import.meta.filename) {
	const options = journeyArguments(
		'O5 short human output, saved requests/models/trees and lint show after rulebook deletion.',
	);
	if (options) await runReportJourney(options);
}
