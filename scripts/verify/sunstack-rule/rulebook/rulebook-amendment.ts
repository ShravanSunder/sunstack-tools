/** R11/R18/R44 through a signed declared CLI, with only synthetic loopback answers. */
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import net from 'node:net';
import path from 'node:path';

import {
	arrayValue,
	configName,
	configure,
	configureEndpoint,
	equal,
	fixtureSource,
	objectValue,
	prepareFixture,
	readObject,
	runSequence,
	requireCondition,
	rulesName,
	selectWorkedSkill,
	writeJson,
	type JourneyArguments,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { declaredTarget, responseBody, RulebookProcess } from './rulebook-process.ts';

export function requireDebugSignature(binary: string): void {
	if (process.platform !== 'darwin') return;
	const signature = spawnSync('codesign', ['-dvv', binary], { encoding: 'utf8', timeout: 10000 });
	const verified = spawnSync('codesign', ['--verify', binary], {
		encoding: 'utf8',
		timeout: 10000,
	});
	requireCondition(
		signature.status === 0 &&
			verified.status === 0 &&
			signature.stderr.includes('Identifier=dev.shravansunder.sunstack-rule.debug') &&
			/TeamIdentifier=(?!not set)\S+/.test(signature.stderr) &&
			signature.stderr.includes('Authority=Developer ID Application:'),
		'Declared binary lacks the required verified Developer ID debug signature',
	);
}
function routeFixture(destination: string): string {
	const root = prepareFixture(destination, ['K001']);
	selectWorkedSkill(root);
	writeJson(path.join(root, 'evidence/rules/selected-step.yml'), {
		id: 'selected-step',
		language: 'Markdown',
		files: ['**/SKILL.md'],
		rule: { all: [{ matches: 'step' }, { regex: '^### 3\\. Choose each observation' }] },
	});
	configure(root, (config) => {
		const evidence = objectValue(config['evidence']);
		const step = objectValue(objectValue(evidence['kinds'])['step']);
		step['rule'] = 'selected-step';
		evidence['kinds'] = { step };
	});
	writeJson(path.join(root, rulesName), {
		K001: {
			name: 'uncertain_handoff',
			evidence: 'step',
			questions: {
				gate: { type: 'noul', instructions: { check: 'body', task: 'Is it eligible?' } },
				verdict: { type: 'noul', instructions: { check: 'body', task: 'Does it comply?' } },
			},
			decision: {
				start: 'gate',
				steps: {
					gate: {
						ask: 'gate',
						yes_at_or_above: 0.7,
						no_at_or_below: 0.3,
						yes: 'verdict',
						['no']: 'not_applicable',
						uncertain: 'verdict',
					},
					verdict: {
						ask: 'verdict',
						yes_at_or_above: 0.7,
						no_at_or_below: 0.3,
						yes: 'pass',
						['no']: 'not_applicable',
					},
				},
			},
		},
	});
	return root;
}
function neighbourFixture(destination: string): string {
	filesystem.cpSync(path.join(fixtureSource, 'neighbours'), destination, { recursive: true });
	return destination;
}

async function observeNeighbours(observer: RulebookProcess, evidence: string): Promise<void> {
	const root = neighbourFixture(path.join(evidence, 'neighbours'));
	configureEndpoint(root, observer.endpoint);
	const result = await observer.report({
		root,
		name: 'neighbour-evidence',
		arguments: ['lint', '--config', configName, '--evidence', 'subject', '--all'],
		expectedExit: 0,
	});
	const listing = arrayValue(result.report['pieces']).map(objectValue);
	equal(listing.length, 1, 'Expected exactly one subject');
	const piece = listing[0];
	requireCondition(piece, 'Subject missing');
	const fields = objectValue(piece['fields']);
	const matches = arrayValue(readObject(path.join(root, 'scan-oracle.json'))['matches']).map(
		objectValue,
	);
	for (const [field, index] of [
		['previous_lists', 0],
		['next_lists', 1],
	] as const) {
		const expected = matches[index];
		requireCondition(expected, 'Independent scan match absent');
		const value = objectValue(fields[field]);
		equal(value['values'], [expected['text']], 'Neighbour text differs from native scan');
		const spans = arrayValue(value['spans']).map(objectValue);
		equal(spans.length, 1, 'Neighbour span missing');
		const span = spans[0];
		requireCondition(span, 'Span missing');
		equal(
			[span['byte_start'], span['byte_end']],
			[expected['start'], expected['end']],
			'Neighbour byte span differs',
		);
	}
	equal(result.captures.length, 0, 'Evidence listing dispatched');
}
async function closedEndpoint(): Promise<string> {
	const server = net.createServer();
	await new Promise<void>((resolve, reject) => {
		server.once('error', reject);
		server.listen(0, '127.0.0.1', resolve);
	});
	const address = server.address();
	requireCondition(address && typeof address === 'object', 'Missing owned port');
	await new Promise<void>((resolve, reject) =>
		server.close((error) => (error ? reject(error) : resolve())),
	);
	return `http://127.0.0.1:${address.port}/systemone`;
}
async function observeRoutesAndReasons(observer: RulebookProcess, evidence: string): Promise<void> {
	const root = routeFixture(path.join(evidence, 'routed'));
	configureEndpoint(root, observer.endpoint);
	await runSequence(
		[
			[0.9, 'pass', 0],
			[0.5, 'uncertain', 5],
			[0.1, 'not_applicable', 0],
		] as const,
		async ([probability, outcome, exit]) => {
			observer.reply = (body) => {
				const reply = responseBody(body);
				const answers = objectValue(reply['answers']);
				objectValue(answers['gate'])['noul'] = 0.5;
				objectValue(answers['verdict'])['noul'] = probability;
				return reply;
			};
			const result = await observer.report({
				root,
				name: `handoff-${probability}`,
				arguments: ['lint', '--config', configName, '--all', '--format', 'json'],
				expectedExit: exit,
				credential: true,
			});
			const results = arrayValue(result.report['results']).map(objectValue);
			equal(results.length, 1, 'Expected one routed pair');
			equal(results[0]?.['outcome'], outcome, 'Wrong routed outcome');
			equal(results[0]?.['step'], 'verdict', 'Pair reports the wrong final step');
			equal(result.captures.length, 1, 'Handoff sent another request');
			equal(
				Object.keys(objectValue(result.captures[0]?.body['questions'])).toSorted(),
				['gate', 'verdict'],
				'Not all questions were in the one request',
			);
			if (outcome === 'uncertain') {
				const uncertain = arrayValue(result.report['uncertain']).map(objectValue);
				equal(uncertain.length, 1, 'Uncertain view missing');
				equal(
					uncertain[0]?.['step'],
					results[0]?.['step'],
					'Uncertain views disagree on final step',
				);
			}
		},
	);
	const endpoint = await closedEndpoint();
	configureEndpoint(root, endpoint);
	const connection = await observer.report({
		root,
		name: 'connection-json',
		arguments: ['lint', '--config', configName, '--all', '--format', 'json'],
		expectedExit: 3,
		credential: true,
	});
	const incomplete = arrayValue(connection.report['incomplete']).map(objectValue);
	equal(incomplete.length, 1, 'Connection case must yield one incomplete pair');
	equal(
		incomplete[0]?.['reason'],
		{ kind: 'provider_failure', failure_class: 'connection' },
		'Connection reason dropped class or fabricated a status',
	);
	equal(connection.captures.length, 0, 'Closed port reached the positive observer');
	const formatted = await observer.run({
		root,
		name: 'connection-default',
		arguments: ['lint', '--config', configName, '--all'],
		expectedExit: 3,
		credential: true,
	});
	requireCondition(
		Buffer.concat([formatted.stdout, formatted.stderr])
			.toString('utf8')
			.includes('provider failure (connection)'),
		'Default line omits connection class',
	);
	configureEndpoint(root, observer.endpoint);
	observer.reply = responseBody;
	const positive = await observer.report({
		root,
		name: 'post-connection-positive',
		arguments: ['lint', '--config', configName, '--all', '--format', 'json'],
		expectedExit: 0,
		credential: true,
	});
	equal(
		positive.captures.length,
		1,
		'Signed positive endpoint did not respond after closed-port probe',
	);
	writeJson(path.join(evidence, 'closed-port.json'), {
		endpoint,
		listener_closed_before_run: true,
		positive_requests_before: 1,
		positive_requests_after: positive.captures.length,
		real_firewall_hold: 'unobserved',
	});
}
async function observeRefusals(observer: RulebookProcess, evidence: string): Promise<void> {
	const definitions: [string, JsonObject][] = [
		['count-zero', { from: 'next', rule: 'neighbour-list', count: 0 }],
		['count-six', { from: 'previous', rule: 'neighbour-list', count: 6 }],
		['kind-only', { from: 'next', kind: 'subject' }],
		['rule-and-kind', { from: 'previous', rule: 'neighbour-list', kind: 'subject' }],
	];
	await runSequence(definitions, async ([name, source]) => {
		const root = neighbourFixture(path.join(evidence, name));
		configure(root, (config) => {
			const subject = objectValue(objectValue(objectValue(config['evidence'])['kinds'])['subject']);
			objectValue(subject['fields'])['next_lists'] = source;
		});
		const result = await observer.run({
			root,
			name,
			arguments: ['lint', '--config', configName, '--check'],
			expectedExit: 2,
		});
		const text = Buffer.concat([result.stdout, result.stderr]).toString('utf8');
		requireCondition(
			text.includes(`${path.join(root, configName)}:`) &&
				/\.json:[1-9]\d*:/.test(text) &&
				text.includes(name.startsWith('count') ? 'neighbour count' : 'unknown field `kind`'),
			'Field refusal lacks its intended cause and config file:line',
		);
		equal(result.captures.length, 0, 'Refusal dispatched');
	});
	await runSequence(
		[
			['undefined-uncertain', 'missing'],
			['cyclic-uncertain', 'gate'],
		] as const,
		async ([name, target]) => {
			const root = routeFixture(path.join(evidence, name));
			const entries = readObject(path.join(root, rulesName));
			const gate = objectValue(
				objectValue(objectValue(objectValue(entries['K001'])['decision'])['steps'])['gate'],
			);
			gate['uncertain'] = target;
			writeJson(path.join(root, rulesName), entries);
			const result = await observer.run({
				root,
				name,
				arguments: ['lint', '--config', configName, '--check'],
				expectedExit: 2,
			});
			const text = Buffer.concat([result.stdout, result.stderr]).toString('utf8');
			requireCondition(
				text.includes(`${path.join(root, rulesName)}:`) &&
					/\.json:[1-9]\d*:/.test(text) &&
					text.includes(target === 'missing' ? 'missing step' : 'cycle'),
				'Route refusal lacks rules file:line',
			);
			equal(result.captures.length, 0, 'Route refusal dispatched');
		},
	);
}
export async function runAmendmentJourney(
	options: JourneyArguments,
	refusals = false,
): Promise<void> {
	requireCondition(!options.prepare, 'Amendment observation requires a declared signed target');
	const target = declaredTarget({
		binary: options.binary ?? '',
		source: options['source-root'],
		commit: options.commit ?? '',
		...(options.sha256 ? { digest: options.sha256 } : {}),
	});
	requireDebugSignature(target.binary);
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		if (refusals) await observeRefusals(observer, evidence);
		else {
			await observeNeighbours(observer, evidence);
			await observeRoutesAndReasons(observer, evidence);
		}
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		commit: target.commit,
		binary_sha256: target.digest,
		claims: refusals
			? ['U2 located field refusals', 'U5 located route refusals']
			: [
					'U2 exact neighbour fields/spans',
					'U5 final step and one request',
					'U18 connection JSON/default',
				],
		live_provider: 'unobserved',
		firewall_hold: 'unobserved',
	});
}
