/** S2 nudge admission: one changed construct per otherwise-valid fixture, no key or backend. */
import filesystem from 'node:fs';
import path from 'node:path';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	configName,
	configure,
	configureEndpoint,
	equal,
	objectValue,
	readObject,
	requireCondition,
	rulesName,
	runSequence,
	writeJson,
	type JourneyArguments,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { nudgeOracle, selectedFixture } from './rulebook-nudge-fixtures.ts';
import { declaredTarget, RulebookProcess } from './rulebook-process.ts';

interface RefusalCase {
	name: string;
	file: string;
	names: readonly string[];
	change: (root: string) => void;
}
function changeEntry(root: string, change: (entry: JsonObject) => void): void {
	const entries = readObject(path.join(root, rulesName));
	change(objectValue(entries['K001']));
	writeJson(path.join(root, rulesName), entries);
}
function findingRoute(entry: JsonObject): JsonObject {
	return objectValue(
		objectValue(objectValue(objectValue(entry['decision'])['steps'])['gate'])['yes'],
	);
}
function changeKind(root: string, name: string, change: (kind: JsonObject) => void): void {
	configure(root, (config) =>
		change(objectValue(objectValue(objectValue(config['evidence'])['kinds'])[name])),
	);
}
function refusalCases(): RefusalCase[] {
	const cases: RefusalCase[] = [
		{
			name: 'related-unknown',
			file: rulesName,
			names: ['nudge.location', 'unknown_field'],
			change: (root) =>
				changeEntry(root, (entry) => {
					entry['related'] = ['unknown_field'];
				}),
		},
		{
			name: 'locator-no-at',
			file: rulesName,
			names: ['nudge.location', 'at'],
			change: (root) =>
				changeEntry(root, (entry) => {
					delete findingRoute(entry)['at'];
				}),
		},
		{
			name: 'locator-question-collision',
			file: rulesName,
			names: ['nudge.location', 'gate'],
			change: (root) =>
				changeEntry(root, (entry) => {
					objectValue(findingRoute(entry)['locate'])['name'] = 'gate';
				}),
		},
		{
			name: 'locator-route-collision',
			file: rulesName,
			names: ['nudge.location', 'anchor_line'],
			change: (root) =>
				changeEntry(root, (entry) => {
					objectValue(objectValue(objectValue(entry['decision'])['steps'])['gate'])['no'] =
						structuredClone(findingRoute(entry));
				}),
		},
		{
			name: 'json-yaml',
			file: configName,
			names: ['question', 'bad_yaml'],
			change: (root) =>
				changeKind(root, 'question', (kind) => {
					objectValue(kind['fields'])['bad_yaml'] = { from: 'yaml', key: 'description' };
				}),
		},
		{
			name: 'json-cross-language',
			file: configName,
			names: ['question', 'cross_language'],
			change: (root) =>
				changeKind(root, 'question', (kind) => {
					objectValue(kind['fields'])['cross_language'] = { from: 'inside', rule: 'all-section' };
				}),
		},
		{
			name: 'target-exclude-kind',
			file: configName,
			names: ['section', 'targets'],
			change: (root) =>
				changeKind(root, 'section', (kind) => {
					objectValue(objectValue(kind['fields'])['targets'])['exclude'] = { kind: 'section' };
				}),
		},
	];
	for (const probability of [0, -0.1, 1.1])
		cases.push({
			name: `locator-probability-${probability}`,
			file: rulesName,
			names: ['nudge.location', 'min_probability'],
			change: (root) =>
				changeEntry(root, (entry) => {
					objectValue(findingRoute(entry)['locate'])['min_probability'] = probability;
				}),
		});
	return cases;
}
function allSources(root: string): void {
	changeKind(root, 'question', (kind) => {
		kind['fields'] = {
			first: { from: 'first_line' },
			captured: { from: 'capture', regex: '"([^"]+)"' },
			inside: { from: 'inside', rule: 'json-pair' },
			enclosing: { from: 'enclosing', rule: 'json-object' },
			same_file: { from: 'same_file', rule: 'json-pair' },
			next: { from: 'next', rule: 'json-pair' },
			previous: { from: 'previous', rule: 'json-pair' },
			body: { from: 'remainder', exclude: { rule: 'json-pair' } },
			targets: {
				from: 'target',
				regex: '"([^" ]+\\.md)"',
				resolve: 'path',
				as_kind: 'gate',
				exclude: { rule: 'json-pair' },
			},
		};
	});
}
export async function runNudgeRefusals(options: JourneyArguments): Promise<void> {
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
	const fixtures = path.join(evidence, 'fixtures');
	filesystem.mkdirSync(fixtures);
	const cases = refusalCases();
	for (const entry of cases) {
		const root = selectedFixture(path.join(fixtures, entry.name), ['K001']);
		entry.change(root);
	}
	const clean = selectedFixture(path.join(fixtures, 'clean'), ['K001']);
	const accepted = selectedFixture(path.join(fixtures, 'all-json-sources'), ['K001']);
	allSources(accepted);
	changeEntry(accepted, (entry) => {
		objectValue(findingRoute(entry)['locate'])['min_probability'] = 1;
	});
	writeJson(
		path.join(evidence, 'cases.json'),
		cases.map((entry) => ({
			name: entry.name,
			file: entry.file,
			diagnostic_names: entry.names,
			expected_exit: 2,
		})),
	);
	if (!target) {
		writeJson(path.join(evidence, 'prepared.json'), {
			product_observed: false,
			oracle: nudgeOracle,
			refusals: cases.length,
			accepts: ['clean', 'all-json-sources'],
		});
		return;
	}
	requireDebugSignature(target.binary);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	const results: JsonObject[] = [];
	try {
		await runSequence([clean, accepted], async (root) => {
			configureEndpoint(root, observer.endpoint);
			const result = await observer.run({
				root,
				name: path.basename(root),
				arguments: ['lint', '--config', configName, '--check'],
				expectedExit: 0,
			});
			equal(result.captures.length, 0, 'Accepted check dispatched');
			requireCondition(
				Buffer.concat([result.stdout, result.stderr]).toString('utf8').includes('K001'),
				'Acceptance lost rule listing',
			);
		});
		await runSequence(cases, async (entry) => {
			const root = path.join(fixtures, entry.name);
			configureEndpoint(root, observer.endpoint);
			const result = await observer.run({
				root,
				name: entry.name,
				arguments: ['lint', '--config', configName, '--check'],
				expectedExit: 2,
			});
			equal(result.captures.length, 0, 'Refused check dispatched');
			const text = Buffer.concat([result.stdout, result.stderr]).toString('utf8');
			const file = path.resolve(root, entry.file);
			requireCondition(
				[...text.matchAll(/^(.+?):(\d+)(?::(\d+))?:/gm)].some(
					(match) => path.resolve(match[1] ?? '') === file && Number(match[2]) > 0,
				),
				`${entry.name}: diagnostic did not locate the mutated file`,
			);
			for (const name of entry.names) {
				const identified =
					name === 'nudge.location'
						? text.includes(name) || text.includes('K001')
						: text.includes(name);
				requireCondition(identified, `${entry.name}: diagnostic omits ${name}`);
			}
			if (entry.name.startsWith('locator-'))
				requireCondition(
					text.includes('gate') && text.includes('locate'),
					`${entry.name}: diagnostic omits route/locator`,
				);
			results.push({ case: entry.name, diagnostic: text.trim(), exit: 2, http_requests: 0 });
		});
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		target,
		oracle: nudgeOracle,
		accepts: ['clean', 'all-json-sources'],
		refusals: results,
	});
	process.stdout.write(
		`Nudge check journey passed ${results.length} targeted refusals and all-source acceptance\n`,
	);
}
