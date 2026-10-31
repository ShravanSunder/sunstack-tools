/** Public exec old-key refusal and Unicode raw/model-view separation at the declared binary. */
import filesystem from 'node:fs';
import path from 'node:path';

import { listingPieces, requireRequestPiece } from './rulebook-evidence-checks.ts';
import {
	codePoints,
	configureEndpoint,
	equal,
	errorMessage,
	journeyArguments,
	lintArguments,
	objectValue,
	parseObject,
	prepareFixture,
	requireCondition,
	runSequence,
	selectWorkedSkill,
	skillName,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from './rulebook-process.ts';
function nativeFixture(root: string): JsonObject {
	filesystem.mkdirSync(root);
	writeJson(path.join(root, 'set.json'), {
		schema_version: 1,
		name: 'proof',
		description: 'controlled boundary',
		inputs: { subject: { kind: 'document' } },
		applicability: [],
		common_ruleset_instructions: 'RULE_COMMON',
		rule_paths: ['rules.jsonl'],
	});
	filesystem.writeFileSync(
		path.join(root, 'rules.jsonl'),
		JSON.stringify({
			rule_id: 'plain',
			evidence: [
				{ id: 'target', input: 'subject', select: { kind: 'paragraph', target_id: 'U001' } },
			],
			target: 'target',
			applicability: [],
			questions: {},
			verdict: {
				kind: 'noul',
				question: { type: 'noul', instructions: 'judge plain' },
				threshold: 0.8,
			},
		}) + '\n',
	);
	filesystem.writeFileSync(path.join(root, 'source.md'), 'U001: original content\n');
	return {
		protocol_version: 1,
		operation: 'check_rules',
		input: {
			definition_root: root,
			definitions: {
				kind: 'rule_sets',
				rule_sets: [
					{
						path: 'set.json',
						binding_groups: [
							{
								bindings: { subject: { kind: 'document', repository: 'repo', path: 'source.md' } },
							},
						],
					},
				],
			},
			repositories: [
				{
					key: 'repo',
					root,
					input_paths: [{ kind: 'exact', path: 'source.md' }],
					change_scope: { kind: 'full' },
				},
			],
			payload_schemas: [],
			limits: {
				max_files: 8,
				max_file_bytes: 65536,
				max_total_bytes: 131072,
				max_matches: 20,
				max_transcript_bytes: 65536,
				include_ignored: false,
			},
			output: 'assembled_rules',
		},
	};
}
async function observeNative(observer: RulebookProcess, evidence: string): Promise<void> {
	const root = path.join(evidence, 'native');
	const envelope = nativeFixture(root);
	const header = path.join(root, 'set.json');
	const original = filesystem.readFileSync(header, 'utf8');
	await observer.run({
		root,
		name: 'native-valid',
		arguments: ['exec'],
		expectedExit: 0,
		input: Buffer.from(JSON.stringify(envelope) + '\n'),
	});
	await runSequence(
		[
			{ key: 'rule_sources', value: [] },
			{ key: 'marker_prefix', value: 'K' },
			{ key: 'max_question_words', value: 400 },
		],
		async (entry) => {
			const configuration = parseObject(original);
			configuration[entry.key] = entry.value;
			writeJson(header, configuration);
			const result = await observer.run({
				root,
				name: `native-old-${entry.key}`,
				arguments: ['exec'],
				expectedExit: 2,
				input: Buffer.from(JSON.stringify(envelope) + '\n'),
			});
			equal(result.captures.length, 0, 'Native refusal dispatched');
			requireCondition(
				Buffer.concat([result.stdout, result.stderr]).includes(entry.key),
				'Native refusal did not identify old field',
			);
		},
	);
}
async function observeUnicode(observer: RulebookProcess, evidence: string): Promise<void> {
	const root = prepareFixture(path.join(evidence, 'unicode'), ['K003']);
	selectWorkedSkill(root);
	configureEndpoint(root, observer.endpoint);
	const raw = 'é'.repeat(3500) + 'MIDDLE_TOKEN' + '尾'.repeat(3500);
	filesystem.writeFileSync(
		path.join(root, skillName),
		`---\ndescription: ${JSON.stringify(raw)}\n---\n`,
	);
	const listing = await observer.run({
		root,
		name: 'unicode-evidence',
		arguments: ['lint', '--config', 'skill-rules.config.json', '--evidence', 'skill', '--all'],
		expectedExit: 0,
	});
	const pieces = listingPieces(listing.stdout);
	const piece = pieces[0];
	requireCondition(piece && pieces.length === 1, 'Unicode fixture did not select one skill');
	equal(
		objectValue(objectValue(piece['fields'])['description'])['value'],
		raw,
		'Raw Unicode evidence changed',
	);
	const view = objectValue(piece['model_view']);
	const bounded = objectValue(view['fields'])['description'];
	requireCondition(
		typeof bounded === 'string' && bounded.includes('[cut]') && !bounded.includes('MIDDLE_TOKEN'),
		'Bounded view lost cut or retained middle',
	);
	equal(codePoints(bounded), 6000, 'Bounded Unicode field must have exactly 6000 scalar values');
	const result = await observer.report({
		root,
		name: 'unicode-request',
		arguments: lintArguments('--all'),
		expectedExit: 0,
		credential: true,
	});
	const captured = result.captures[0];
	requireCondition(
		captured && result.captures.length === 1,
		'Unicode fixture did not dispatch one request',
	);
	requireRequestPiece({
		state: objectValue(captured.body['state']),
		piece,
		standard: 'A skill description names when to use the skill.',
		questions: objectValue(captured.body['questions']),
	});
	requireCondition(!captured.raw.includes('MIDDLE_TOKEN'), 'Request leaked cut middle');
	writeJson(path.join(evidence, 'unicode.json'), {
		raw_characters: codePoints(raw),
		model_view_characters: codePoints(bounded),
		http_requests: 1,
	});
}
export async function main(): Promise<void> {
	const options = journeyArguments('Rulebook exec refusal and Unicode model-view boundaries.');
	if (!options) return;
	requireCondition(!options.prepare, 'Use the three fixture journeys for offline preparation');
	const target = declaredTarget({
		binary: options.binary ?? '',
		source: options['source-root'],
		commit: options.commit ?? '',
		...(options.sha256 ? { digest: options.sha256 } : {}),
	});
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		await observeNative(observer, evidence);
		await observeUnicode(observer, evidence);
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		commit: target.commit,
		binary_sha256: target.digest,
		cli_processes: 6,
		http_requests: 1,
	});
	process.stdout.write('Native old-key and Unicode boundary probes passed\n');
}
if (import.meta.main)
	try {
		await main();
	} catch (error) {
		process.stderr.write(`Boundary observation failed: ${errorMessage(error)}\n`);
		process.exitCode = 2;
	}
