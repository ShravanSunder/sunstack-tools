/** V23 compares signed pre-amendment and current binaries through the existing real journeys. */
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	arrayValue,
	configureEndpoint,
	equal,
	errorMessage,
	lintArguments,
	objectValue,
	parseObject,
	readObject,
	runSequence,
	requireCondition,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import {
	observeLegacyRulebook,
	prepareRulebook,
	rulebookFixtureNames,
	scriptedReply,
} from './rulebook-lint.ts';
import { declaredTarget, RulebookProcess, type DeclaredTarget } from './rulebook-process.ts';

export interface CompatibilityCase {
	report: JsonObject;
	exit: number;
	requests: number;
}
/** Every public pair field and both consumer views retain their emitted order. */
export function requireRulebookCompatibility(
	baseline: CompatibilityCase,
	current: CompatibilityCase,
	allowedReason?: JsonObject,
): JsonObject[] {
	equal(current.exit, baseline.exit, 'V23 exit changed');
	equal(current.requests, baseline.requests, 'V23 request count changed');
	for (const field of ['results', 'findings', 'uncertain']) {
		requireCondition(
			Array.isArray(baseline.report[field]) && Array.isArray(current.report[field]),
			`Missing complete ${field} view`,
		);
		equal(current.report[field], baseline.report[field], `V23 ordered ${field} changed`);
	}
	const before = arrayValue(baseline.report['incomplete']).map(objectValue);
	const after = arrayValue(current.report['incomplete']).map(objectValue);
	equal(after.length, before.length, 'V23 incomplete membership changed');
	const changes: JsonObject[] = [];
	for (let index = 0; index < before.length; index++) {
		const oldPart = before[index],
			newPart = after[index];
		requireCondition(oldPart && newPart, 'Incomplete pair absent');
		const { reason: oldReason, ...oldIdentity } = oldPart;
		const { reason: newReason, ...newIdentity } = newPart;
		equal(newIdentity, oldIdentity, 'V23 incomplete identity/order changed');
		if (allowedReason) {
			equal(
				oldReason,
				{ kind: 'provider_failure' },
				'R44 baseline was not the retired bare reason',
			);
			equal(newReason, allowedReason, 'R44 reason differs from its exact failure oracle');
			changes.push({ index, before: oldReason ?? null, after: newReason ?? null });
		} else equal(newReason, oldReason, 'Unlisted incomplete reason changed');
	}
	if (allowedReason)
		requireCondition(changes.length > 0, 'R44 comparison did not reach an incomplete pair');
	return changes;
}
const fixtureExits: Record<(typeof rulebookFixtureNames)[number], number> = {
	'all-kinds': 1,
	worked: 1,
	'uncertain-only': 0,
	skip: 3,
	malformed: 2,
	'missing-completion': 0,
	changes: 0,
	'changed-load': 1,
	'rule-change': 1,
	untracked: 0,
};
async function observe(
	target: DeclaredTarget,
	evidence: string,
): Promise<Record<string, CompatibilityCase>> {
	requireDebugSignature(target.binary);
	filesystem.mkdirSync(evidence);
	const fixtures = path.join(evidence, 'fixtures');
	prepareRulebook(fixtures);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		observer.reply = scriptedReply;
		await runSequence(rulebookFixtureNames, async (name) => {
			const root = path.join(fixtures, name);
			if (name !== 'malformed') configureEndpoint(root, observer.endpoint);
			await observer.report({
				root,
				name: `compat-${name}`,
				arguments: lintArguments('--all'),
				expectedExit: fixtureExits[name],
				credential: true,
			});
		});
		await observeLegacyRulebook(observer, fixtures);
		const root = path.join(fixtures, 'worked');
		await runSequence(['rate-limit', 'invalid-response'] as const, async (fault) => {
			observer.respond = async () =>
				fault === 'rate-limit'
					? { status: 429, body: { error: 'controlled rate limit' } }
					: {
							status: 200,
							body: {
								model: 'typesafe/controlled',
								answers: {},
								usage: { input_tokens: 1, output_tokens: 1, cost: 0.000001 },
							},
						};
			await observer.report({
				root,
				name: `r44-${fault}`,
				arguments: lintArguments('--all'),
				expectedExit: 3,
				credential: true,
			});
		});
	} finally {
		await observer.close();
	}
	const reports: Record<string, CompatibilityCase> = {};
	for (const entry of filesystem.readdirSync(path.join(evidence, 'run'), { withFileTypes: true })) {
		if (!entry.isDirectory()) continue;
		const directory = path.join(evidence, 'run', entry.name);
		const processFile = path.join(directory, 'process.json');
		if (!filesystem.existsSync(processFile)) continue;
		const process = readObject(processFile);
		const stdout = filesystem.readFileSync(path.join(directory, 'stdout.log'), 'utf8');
		if (!stdout.trim().startsWith('{')) continue;
		const report = parseObject(stdout);
		if (!('results' in report)) continue;
		reports[entry.name] = {
			report,
			exit: z.number().parse(process['exit_code']),
			requests: z.number().parse(process['http_requests']),
		};
	}
	requireCondition(
		Object.keys(reports).length >= rulebookFixtureNames.length,
		'Compatibility did not observe all existing fixtures',
	);
	return reports;
}
const argumentsSchema = z.object({
	baseline: z.string(),
	'baseline-sha256': z.string().regex(/^[0-9a-f]{64}$/),
	'baseline-source': z.string(),
	'baseline-commit': z.string().regex(/^[0-9a-f]{40}$/),
	binary: z.string(),
	sha256: z.string().regex(/^[0-9a-f]{64}$/),
	'source-root': z.string(),
	commit: z.string().regex(/^[0-9a-f]{40}$/),
	evidence: z.string(),
});
export async function main(): Promise<void> {
	const parsed = parseArgs({
		options: {
			baseline: { type: 'string' },
			'baseline-sha256': { type: 'string' },
			'baseline-source': { type: 'string' },
			'baseline-commit': { type: 'string' },
			binary: { type: 'string' },
			sha256: { type: 'string' },
			'source-root': { type: 'string' },
			commit: { type: 'string' },
			evidence: { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (parsed.values.help) {
		process.stdout.write(
			'V23: --baseline BINARY --baseline-sha256 HASH --baseline-source CLEAN_CHECKOUT --baseline-commit FULL_SHA --binary BINARY --sha256 HASH --source-root CLEAN_CHECKOUT --commit FULL_SHA --evidence NEW_DIR. Same existing fixtures/scripted answers; ordered complete pair results, findings, uncertain views, exits, HTTP counts. Only two explicit R44 failure-reason differences are allowed; paths remain native-test proof.\n',
		);
		return;
	}
	const options = argumentsSchema.parse(parsed.values);
	const baseline = declaredTarget({
		binary: options.baseline,
		digest: options['baseline-sha256'],
		source: options['baseline-source'],
		commit: options['baseline-commit'],
	});
	const current = declaredTarget({
		binary: options.binary,
		digest: options.sha256,
		source: options['source-root'],
		commit: options.commit,
	});
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const before = await observe(baseline, path.join(evidence, 'baseline'));
	const after = await observe(current, path.join(evidence, 'current'));
	equal(
		Object.keys(after).toSorted(),
		Object.keys(before).toSorted(),
		'Compatibility case inventory changed',
	);
	const changes: JsonObject[] = [];
	for (const [name, reference] of Object.entries(before)) {
		const candidate = after[name];
		requireCondition(candidate, 'Current case missing');
		const allowed =
			name === 'r44-rate-limit'
				? { kind: 'provider_failure', failure_class: 'rate_limit', http_status: 429 }
				: name === 'r44-invalid-response'
					? { kind: 'invalid_response' }
					: undefined;
		for (const change of requireRulebookCompatibility(reference, candidate, allowed))
			changes.push({ case: name, ...change });
	}
	const worked = after['compat-worked'];
	requireCondition(worked, 'Worked report missing');
	const planted = structuredClone(worked);
	const rows = arrayValue(planted.report['results']);
	requireCondition(rows[0], 'Planted pair missing');
	objectValue(rows[0])['step'] = 'planted_wrong_step';
	let rejected = false;
	try {
		requireRulebookCompatibility(worked, planted);
	} catch {
		rejected = true;
	}
	requireCondition(rejected, 'Comparator accepted a planted wrong step');
	writeJson(path.join(evidence, 'pass.json'), {
		baseline,
		current,
		cases: Object.keys(before),
		r44_reason_changes: changes,
		planted_step_rejected: rejected,
		path_taken: 'native walk tests; absent from public report',
	});
	process.stdout.write(
		'V23 compatibility passed with explicit R44 reason deltas and planted rejection\n',
	);
}
if (import.meta.main)
	try {
		await main();
	} catch (error) {
		process.stderr.write(`V23 compatibility failed: ${errorMessage(error)}\n`);
		process.exitCode = 2;
	}
