/** R40 check-only controls, using canonical S0 mutations with no checked-content acquisition. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	configName,
	configure,
	configureEndpoint,
	equal,
	jsonValueSchema,
	arrayValue,
	objectValue,
	requireCondition,
	runSequence,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { type DeclaredTarget, RulebookProcess } from './rulebook-process.ts';
import { copySelectionFixture } from './selection-fixture.ts';

const withinRefusals: Record<string, { change: (kinds: JsonObject) => void; cause: RegExp }> = {
	'amended-within-string': {
		change: (kinds) => {
			objectValue(kinds['step'])['within'] = 'workflow_section';
		},
		cause: /sequence|list|array/i,
	},
	'amended-within-null': {
		change: (kinds) => {
			objectValue(kinds['step'])['within'] = null;
		},
		cause: /null.*(?:sequence|list|array)|(?:sequence|list|array).*null/i,
	},
	'amended-within-empty': {
		change: (kinds) => {
			objectValue(kinds['step'])['within'] = [];
		},
		cause: /empty|at least|min/i,
	},
	'amended-within-repeated': {
		change: (kinds) => {
			objectValue(kinds['step'])['within'] = ['workflow_section', 'workflow_section'];
		},
		cause: /repeat|duplicate/i,
	},
	'amended-within-undefined-second': {
		change: (kinds) => {
			objectValue(kinds['step'])['within'] = ['workflow_section', 'missing_within_kind'];
		},
		cause: /undefined.*missing_within_kind|missing_within_kind.*undefined/i,
	},
	'amended-within-cycle-second': {
		change: (kinds) => {
			objectValue(kinds['step'])['within'] = ['workflow_section', 'within_cycle_peer'];
			kinds['within_cycle_peer'] = {
				rule: 'section',
				within: ['step'],
				fields: { body: { from: 'remainder' } },
			};
		},
		cause: /cycle/i,
	},
	'field-kind-prefix-collision': {
		change: (kinds) => {
			kinds['foo.fields.body'] = { rule: 'section', fields: {} };
			kinds['a'] = { rule: 'section', within: ['foo.fields.body'], fields: {} };
			kinds['a.within.foo'] = {
				rule: 'section',
				fields: { body: { from: 'inside', kind: 'missing_field_kind' } },
			};
		},
		cause: /undefined evidence kind/,
	},
};

const literalKindNames: Record<string, string> = {
	'literal-kind-dotted': 'a.b',
	'literal-kind-collision': 'a.fields.body',
	'literal-kind-ordinary': 'ordinary_kind',
	'literal-kind-escaped': 'odd"\\kind',
};
for (const [name, kind] of Object.entries(literalKindNames)) {
	withinRefusals[name] = {
		change: (kinds) => {
			kinds['a'] = { rule: 'section', fields: { body: { from: 'first_line' } } };
			kinds[kind] = {
				rule: 'section',
				within: ['workflow_section', 'missing_literal_kind'],
				fields: {},
			};
		},
		cause: /undefined kind missing_literal_kind/,
	};
}

/** Authored two-space fixtures put evidence.kinds keys at six-space indentation. */
export function requireKindDiagnosticLocation(options: {
	configText: string;
	kind: string;
	file: string;
	diagnostic: string;
}): void {
	const keyPrefix = '      ' + JSON.stringify(options.kind) + ':';
	const locations = options.configText
		.split('\n')
		.flatMap((line, index) => (line.startsWith(keyPrefix) ? [`${index + 1}:7`] : []));
	equal(locations.length, 1, 'Fixture must contain exactly one owning evidence kind key');
	requireCondition(
		options.diagnostic.includes(`${path.resolve(options.file)}:${locations[0]}:`),
		'Kind-plan diagnostic must locate the owning evidence.kinds.' + options.kind + ' key',
	);
}

/** The controlled fixture's field key must be inside its complete literal kind block. */
export function requireFieldDiagnosticLocation(options: {
	configText: string;
	kind: string;
	field: string;
	file: string;
	diagnostic: string;
}): void {
	const lines = options.configText.split('\n');
	const kindPrefix = '      ' + JSON.stringify(options.kind) + ':';
	const starts = lines.flatMap((line, index) => (line.startsWith(kindPrefix) ? [index] : []));
	equal(starts.length, 1, 'Fixture must contain exactly one owning literal kind block');
	const start = starts[0];
	requireCondition(start !== undefined, 'Owning literal kind block missing');
	const end = lines.findIndex((line, index) => index > start && line.startsWith('      }'));
	requireCondition(end > start, 'Owning literal kind block is not closed');
	const fieldPrefix = '          ' + JSON.stringify(options.field) + ':';
	const locations = lines.flatMap((line, index) =>
		index > start && index < end && line.startsWith(fieldPrefix) ? [`${index + 1}:11`] : [],
	);
	equal(locations.length, 1, 'Fixture must contain exactly one field key in its owning kind');
	requireCondition(
		options.diagnostic.includes(`${path.resolve(options.file)}:${locations[0]}:`),
		'Field-source diagnostic must locate the field inside its complete literal kind',
	);
}

const stepLocationCases = new Set([
	'kind-cycle',
	'within-cycle',
	'within-undefined',
	'amended-within-undefined-second',
	'amended-within-cycle-second',
]);

export function prepareSelectionRefusals(options: {
	evidence: string;
	sourceRoot: string;
}): string[] {
	filesystem.mkdirSync(options.evidence);
	const base = copySelectionFixture({
		destination: path.join(options.evidence, 'clean'),
		sourceRoot: options.sourceRoot,
		allKinds: true,
	});
	const omitted = copySelectionFixture({
		destination: path.join(options.evidence, 'omitted-within'),
		sourceRoot: options.sourceRoot,
		allKinds: true,
	});
	configure(omitted, (config) => {
		delete objectValue(objectValue(objectValue(config['evidence'])['kinds'])['step'])['within'];
	});
	const sources = filesystem
		.readdirSync(path.join(base, 'refusals'))
		.filter((file) => file.endsWith('.json'))
		.toSorted();
	requireCondition(sources.length >= 18, 'S0 R40 refusal inventory missing');
	for (const file of sources) {
		const root = copySelectionFixture({
			destination: path.join(options.evidence, path.basename(file, '.json')),
			sourceRoot: options.sourceRoot,
			allKinds: true,
		});
		filesystem.copyFileSync(path.join(base, 'refusals', file), path.join(root, configName));
	}
	const cases = sources.map((file) => path.basename(file, '.json'));
	for (const [name, refusal] of Object.entries(withinRefusals)) {
		requireCondition(
			!cases.includes(name),
			'Amended refusal name collides with canonical S0 inventory',
		);
		const root = copySelectionFixture({
			destination: path.join(options.evidence, name),
			sourceRoot: options.sourceRoot,
			allKinds: true,
		});
		configure(root, (config) => {
			refusal.change(objectValue(objectValue(config['evidence'])['kinds']));
		});
		cases.push(name);
	}
	writeJson(path.join(options.evidence, 'prepared.json'), {
		product_observed: false,
		cases,
		oracle: 'docs/specs/2026-10-30-evidence-selection/specification.md R40',
	});
	return cases;
}
export async function observeSelectionRefusals(options: {
	evidence: string;
	sourceRoot: string;
	target: DeclaredTarget;
}): Promise<void> {
	const cases = prepareSelectionRefusals(options);
	const observer = new RulebookProcess(options.target, path.join(options.evidence, 'run'));
	await observer.start();
	const results: JsonObject[] = [];
	try {
		const clean = path.join(options.evidence, 'clean');
		configureEndpoint(clean, observer.endpoint);
		const checked = await observer.run({
			root: clean,
			name: 'selection-clean-check',
			arguments: ['lint', '--config', configName, '--check'],
			expectedExit: 0,
		});
		equal(checked.captures.length, 0, 'Clean check dispatched');
		const text = Buffer.concat([checked.stdout, checked.stderr]).toString('utf8');
		for (const name of ['workflow_section', 'step'])
			requireCondition(text.includes(name), 'Check listing omits kind ' + name);
		const listing = arrayValue(
			jsonValueSchema.parse(JSON.parse(checked.stdout.toString('utf8'))),
		).map(objectValue);
		equal(
			listing.find((row) => row['kind'] === 'step')?.['within'],
			['workflow_section'],
			'One-item containment listing is not the new list contract',
		);
		const omitted = path.join(options.evidence, 'omitted-within');
		configureEndpoint(omitted, observer.endpoint);
		const omittedCheck = await observer.run({
			root: omitted,
			name: 'selection-omitted-within-check',
			arguments: ['lint', '--config', configName, '--check'],
			expectedExit: 0,
		});
		equal(omittedCheck.captures.length, 0, 'Omitted within control dispatched');
		const omittedListing = arrayValue(
			jsonValueSchema.parse(JSON.parse(omittedCheck.stdout.toString('utf8'))),
		).map(objectValue);
		requireCondition(
			!omittedListing.some((row) => row['kind'] === 'step'),
			'An omitted within must leave the plain step kind without a containment listing',
		);
		await runSequence(cases, async (name) => {
			const root = path.join(options.evidence, name);
			const file = path.join(root, configName);
			if (withinRefusals[name]) configureEndpoint(root, observer.endpoint);
			else
				filesystem.writeFileSync(
					file,
					filesystem
						.readFileSync(file, 'utf8')
						.replace(
							'"model": "jev-latest"',
							`"model": "jev-latest", "endpoint": "${observer.endpoint}"`,
						),
				);
			const result = await observer.run({
				root,
				name: 'r40-' + name,
				arguments: ['lint', '--config', configName, '--check'],
				expectedExit: 2,
			});
			equal(result.captures.length, 0, 'R40 refusal dispatched');
			const diagnostic = Buffer.concat([result.stdout, result.stderr]).toString('utf8');
			requireCondition(
				diagnostic.includes(path.resolve(file)) && new RegExp(':\\d+(?::\\d+)?:').test(diagnostic),
				'R40 refusal lacks config file/line',
			);
			const amended = withinRefusals[name];
			if (amended) {
				requireCondition(
					amended.cause.test(diagnostic),
					'Within refusal did not reach its intended cause: ' + name,
				);
				if (name === 'amended-within-cycle-second') {
					for (const kind of ['step', 'within_cycle_peer'])
						requireCondition(
							diagnostic.includes(kind),
							'Cycle diagnostic omits the dependency path kind ' + kind,
						);
				}
			}
			const owningKind =
				literalKindNames[name] ?? (stepLocationCases.has(name) ? 'step' : undefined);
			if (owningKind)
				requireKindDiagnosticLocation({
					configText: filesystem.readFileSync(file, 'utf8'),
					kind: owningKind,
					file,
					diagnostic,
				});
			if (name === 'field-kind-prefix-collision') {
				const rows = diagnostic.split('\n');
				const fieldRows = rows.filter((row) => row.endsWith(': undefined evidence kind'));
				const plannerRows = rows.filter((row) =>
					row.endsWith(': undefined kind missing_field_kind'),
				);
				equal(fieldRows.length, 1, 'Field collision must retain its field-source refusal');
				equal(plannerRows.length, 1, 'Field collision must retain its planner refusal');
				const configText = filesystem.readFileSync(file, 'utf8');
				requireFieldDiagnosticLocation({
					configText,
					kind: 'a.within.foo',
					field: 'body',
					file,
					diagnostic: fieldRows.join('\n'),
				});
				requireKindDiagnosticLocation({
					configText,
					kind: 'a.within.foo',
					file,
					diagnostic: plannerRows.join('\n'),
				});
			}
			results.push({ case: name, exit: 2, requests: 0, diagnostic });
		});
	} finally {
		await observer.close();
	}
	writeJson(path.join(options.evidence, 'pass.json'), {
		source_commit: options.target.commit,
		binary_sha256: options.target.digest,
		cases: results,
		accepted_controls: ['omitted within', 'one-item within; two-item control in union journey'],
		required_join:
			'Check signature/call path has no checked-source/capture/backend; native R40 admission assertions distinguish exact causes.',
	});
}
