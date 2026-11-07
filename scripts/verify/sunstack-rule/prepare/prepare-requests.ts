/** Native --prepare/report/raw-HTTP parity. No paid credentials or product builders are used. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireDebugSignature } from '../rulebook/rulebook-amendment.ts';
import {
	arrayValue,
	configureEndpoint,
	objectValue,
	numberValue,
	parseObject,
	writeJson,
	runSequence,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess, responseBody } from '../rulebook/rulebook-process.ts';
import { observePrepareCases } from './prepare-cases.ts';
import {
	requirePreparedGate,
	requireRequestParity,
	requireSelectionWaits,
} from './prepare-checks.ts';
import {
	prepareConfig,
	prepareRules,
	prepareRequestFixture,
	prepareSelectionFixture,
} from './prepare-fixtures.ts';
import { compactJson, jsonSlice } from './raw-json.ts';
import { observeSelectionReplay } from './selection-replay.ts';

const optionSchema = z.object({
	binary: z.string(),
	'source-root': z.string(),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	evidence: z.string(),
	'prepare-exit': z.coerce.number().int().min(0).max(255),
});
function ordinaryReply(body: JsonObject, admitAll: boolean): JsonObject {
	const reply = responseBody(body, 0.95);
	if ('category' in objectValue(body['questions'])) {
		const description = objectValue(body['state'])['description'];
		const selected =
			!admitAll && description === 'PRODUCT_FILTER_SENTINEL' ? 'product' : 'portable';
		const answers = objectValue(reply['answers']);
		answers['category'] = {
			type: 'choice',
			choice: selected,
			confidence: 1,
			probabilities: {
				portable: selected === 'portable' ? 1 : 0,
				product: selected === 'product' ? 1 : 0,
			},
		};
	}
	return reply;
}
export async function observePrepare(options: z.infer<typeof optionSchema>): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	requireDebugSignature(target.binary);
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const root = path.join(evidence, 'requests-fixture');
	prepareRequestFixture(root);
	const selectionRoot = path.join(evidence, 'selection-fixture');
	prepareSelectionFixture(selectionRoot, target.source);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		configureEndpoint(root, observer.endpoint);
		configureEndpoint(selectionRoot, observer.endpoint);
		await runSequence([...[root, selectionRoot].entries()], async ([index, fixture]) => {
			const check = await observer.run({
				root: fixture,
				name: `check-${index}`,
				arguments: ['lint', '--config', prepareConfig, '--check'],
				expectedExit: 0,
			});
			assert.equal(check.captures.length, 0);
		});
		const preparedFile = path.join(evidence, 'requests.prepared.json');
		const prepared = await observer.run({
			root,
			name: 'prepare-requests',
			arguments: ['lint', '--config', prepareConfig, '--prepare', preparedFile, '--all'],
			expectedExit: options['prepare-exit'],
		});
		assert.equal(prepared.captures.length, 0, 'Prepare dispatched HTTP without a credential');
		const raw = filesystem.readFileSync(preparedFile, 'utf8');
		requirePreparedGate(raw, 'prepared_gated');
		await runSequence(
			[
				['gate-excludes-product', false],
				['gate-admits-all', true],
			] as const,
			async ([name, admitAll]) => {
				observer.respond = ({ body }) =>
					Promise.resolve({ status: 200, body: ordinaryReply(body, admitAll) });
				const reportFile = path.join(evidence, `${name}.report.json`);
				const ordinary = await observer.run({
					root,
					name,
					arguments: [
						'lint',
						'--config',
						prepareConfig,
						'--report',
						reportFile,
						'--format',
						'json',
						'--all',
					],
					credential: true,
					expectedExit: 0,
				});
				assert.equal(
					ordinary.captures.length,
					admitAll ? 10 : 8,
					'Ordinary gate did not control the independent request inventory',
				);
				requireRequestParity({
					prepared: raw,
					ordinary: filesystem.readFileSync(reportFile, 'utf8'),
					captures: ordinary.captures.map((capture) => capture.raw.toString('utf8')),
					expectedPrepared: 10,
				});
			},
		);
		const saved = parseObject(raw);
		assert.ok(
			!JSON.stringify(saved['requests']).includes('"answers"'),
			'Prepared requests contain an answer map',
		);
		const evidenceText = JSON.stringify(saved['piece_evidence']);
		assert.ok(
			evidenceText.includes('MIDDLE_SHOULD_BE_CUT'),
			'Prepared report lost unprojected evidence',
		);
		const requests = arrayValue(saved['requests']).map(objectValue);
		const ruleRequests = requests.filter(
			(request) => objectValue(request['owner'])['kind'] === 'rule',
		);
		assert.equal(ruleRequests.length, 8);
		assert.ok(
			ruleRequests.some((request) => {
				const payload = objectValue(request['request']);
				return (
					'focus_line' in objectValue(payload['questions']) &&
					'locator_context' in objectValue(payload['state'])
				);
			}),
			'Locator-only field was not projected beside its question',
		);
		const selectedFile = path.join(evidence, 'selection.prepared.json');
		const selection = await observer.run({
			root: selectionRoot,
			name: 'prepare-selection',
			arguments: ['lint', '--config', prepareConfig, '--prepare', selectedFile, '--all'],
			expectedExit: options['prepare-exit'],
		});
		assert.equal(selection.captures.length, 0);
		const selectedRaw = filesystem.readFileSync(selectedFile, 'utf8');
		requireSelectionWaits(selectedRaw);
		observer.respond = ({ body }) =>
			Promise.resolve({ status: 200, body: responseBody(body, 0.95) });
		const selectionReportFile = path.join(evidence, 'selection.ordinary.json');
		const ordinarySelection = await observer.run({
			root: selectionRoot,
			name: 'selection-sends',
			arguments: [
				'lint',
				'--config',
				prepareConfig,
				'--report',
				selectionReportFile,
				'--format',
				'json',
				'--all',
			],
			credential: true,
			expectedExit: 0,
		});
		const selectionCaptures = ordinarySelection.captures.filter(
			(capture) => 'candidates' in objectValue(capture.body['state']),
		);
		assert.ok(selectionCaptures.length > 0, 'Ordinary selection listener witness was inert');
		const preparedSelections = arrayValue(parseObject(selectedRaw)['requests']).map(objectValue);
		for (const capture of selectionCaptures)
			assert.ok(
				preparedSelections.some((_record, index) =>
					['state', 'questions'].every(
						(field) =>
							compactJson(jsonSlice(selectedRaw, ['requests', index, 'request', field])) ===
							compactJson(jsonSlice(capture.raw.toString('utf8'), [field])),
					),
				),
				'Prepared selection differs from actual dispatched bytes',
			);
		await observeSelectionReplay({
			observer,
			root: selectionRoot,
			evidence,
			ordinaryFile: selectionReportFile,
			unansweredFile: selectedFile,
			captures: ordinarySelection.captures,
			prepareExit: options['prepare-exit'],
		});
		await observePrepareCases({ observer, evidence });
		const first = objectValue(arrayValue(saved['results'])[0]);
		const selectedId = String(numberValue(first['id']));
		const show = await observer.run({
			root,
			name: 'show-prepared',
			arguments: ['lint', 'show', preparedFile, selectedId],
			expectedExit: 0,
		});
		assert.equal(show.captures.length, 0);
		const shown = parseObject(show.stdout.toString('utf8'));
		assert.equal(objectValue(shown['result'])['id'], first['id']);
		filesystem.unlinkSync(path.join(root, prepareRules));
		filesystem.unlinkSync(path.join(root, 'rulebook.md'));
		const afterRemoval = await observer.run({
			root,
			name: 'show-without-sources',
			arguments: ['lint', 'show', preparedFile, selectedId],
			expectedExit: 0,
		});
		assert.equal(afterRemoval.captures.length, 0);
		assert.deepEqual(
			afterRemoval.stdout,
			show.stdout,
			'Prepared show depended on current source files',
		);

		writeJson(path.join(evidence, 'pass.json'), {
			target,
			prepare_exit: options['prepare-exit'],
			prepared_request_count: 10,
			selection_requests: selectionCaptures.length,
			product_observed: true,
			no_prepare_http: true,
			limitations: [
				'Controlled provider mechanics only; case/edge coverage is accounted separately in the observation report.',
			],
		});
	} finally {
		await observer.close();
	}
}
if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: {
			binary: { type: 'string' },
			'source-root': { type: 'string' },
			commit: { type: 'string' },
			sha256: { type: 'string' },
			evidence: { type: 'string' },
			'prepare-exit': { type: 'string' },
			help: { type: 'boolean' },
		},
		strict: true,
	});
	if (values.help)
		process.stdout.write(
			'Observe native --prepare with byte-preserving persisted/report/raw HTTP parity.\n--binary PATH --source-root CHECKOUT --commit FULL_SHA --sha256 DIGEST --prepare-exit DECLARED_CODE --evidence NEW_DIRECTORY\nUses only owned loopback replies and synthetic credentials on ordinary neighbor runs. Prepare/show are credential-free. Fixtures, raw captures and exits are retained. No paid/model calls.\n',
		);
	else await observePrepare(optionSchema.parse(values));
}
