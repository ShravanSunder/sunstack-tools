/** Keyless real HTML and labelled controls through one declared signed native binary. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireDebugSignature } from '../rulebook/rulebook-amendment.ts';
import {
	arrayValue,
	objectValue,
	readObject,
	runSequence,
	textValue,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import {
	realAskingText,
	requireAsking,
	requireControls,
	requireNoDispatch,
	requireRealCoverage,
	savedPiece,
} from './html-checks.ts';
import { corpusRevision } from './html-corpus.ts';
import {
	htmlConfig,
	prepareAnchorFixture,
	prepareAskingFixture,
	prepareGapFixture,
	prepareRealAsking,
	prepareRealFixture,
	prepareRealForward,
	prepareTokenFixture,
	type ExpectedControl,
} from './html-fixtures.ts';

const optionsSchema = z.object({
	binary: z.string(),
	'source-root': z.string(),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	evidence: z.string(),
});
export async function observeHtml(options: z.infer<typeof optionsSchema>): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	requireDebugSignature(target.binary);
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const real = path.join(evidence, 'real-reverse');
	prepareRealFixture({ root: real, source: target.source });
	const forward = path.join(evidence, 'real-forward');
	prepareRealForward({ root: forward, source: target.source });
	const realAsking = path.join(evidence, 'real-asking');
	prepareRealAsking({ root: realAsking, source: target.source });
	const asking = path.join(evidence, 'labelled-asking');
	prepareAskingFixture(asking);
	const controls: { name: string; root: string; expected: ExpectedControl[]; exit: number }[] = [];
	for (const [name, prepare, exit] of [
		['token', prepareTokenFixture, 1],
		['anchor', prepareAnchorFixture, 3],
		['empty-needle', (root: string) => prepareGapFixture(root, 'empty-needle'), 3],
		['missing-document', (root: string) => prepareGapFixture(root, 'missing-document'), 3],
		['empty-target', (root: string) => prepareGapFixture(root, 'empty-target'), 3],
	] as const) {
		const root = path.join(evidence, `labelled-${name}`);
		controls.push({ name, root, expected: prepare(root), exit });
	}
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		await runSequence(
			[real, forward, realAsking, asking, ...controls.map((control) => control.root)],
			async (root) => {
				const configFile = path.join(root, htmlConfig);
				const config = readObject(configFile);
				config['decision_model'] = {
					...objectValue(config['decision_model']),
					endpoint: observer.endpoint,
				};
				writeJson(configFile, config);
				const check = await observer.run({
					root,
					name: `check-${path.basename(root)}`,
					arguments: ['lint', '--config', htmlConfig, '--check'],
					expectedExit: 0,
				});
				assert.equal(check.captures.length, 0);
			},
		);
		const prepare = async (
			root: string,
			name: string,
			expectedExit: number,
		): Promise<ReturnType<typeof readObject>> => {
			const reportFile = path.join(evidence, `${name}.prepared.json`);
			const result = await observer.run({
				root,
				name,
				arguments: ['lint', '--config', htmlConfig, '--prepare', reportFile, '--all'],
				expectedExit,
			});
			assert.equal(result.captures.length, 0, `${name} unexpectedly dispatched`);
			return readObject(reportFile);
		};
		const reverse = await prepare(real, 'real-reverse', 6);
		writeJson(path.join(evidence, 'reverse-account.json'), requireRealCoverage(reverse, real));
		const forwardReport = await prepare(forward, 'real-forward', 6);
		requireNoDispatch(forwardReport);
		assert.deepEqual(
			arrayValue(forwardReport['results']),
			[],
			'Real-set fragment inventory should be zero',
		);
		assert.deepEqual(arrayValue(forwardReport['requests']), []);
		requireAsking(await prepare(realAsking, 'real-asking', 6), realAsking, realAskingText);
		requireAsking(await prepare(asking, 'labelled-asking', 6), asking, 'Café & Unicode 文 body.');
		await runSequence(controls, async (control) => {
			const report = await prepare(control.root, control.name, control.exit);
			requireControls(report, control.expected);
			if (['empty-needle', 'missing-document', 'empty-target'].includes(control.name)) {
				const row = objectValue(arrayValue(report['results'])[0]);
				const fields = objectValue(savedPiece(report, row)['fields']);
				if (control.name === 'empty-needle') {
					const needle = objectValue(fields['identifier']);
					assert.equal(
						needle['state'],
						'absent',
						'Capture excludes empty groups by its existing contract',
					);
					assert.ok(textValue(savedPiece(report, row)['text']).includes('data-needle=""'));
					const gaps = arrayValue(report['incomplete']).map(objectValue);
					assert.equal(gaps.length, 1);
					assert.equal(gaps[0]?.['field'], 'identifier');
				} else {
					const field = objectValue(fields[control.name === 'empty-target' ? 'links' : 'coverage']);
					assert.equal(field['state'], 'unavailable');
					assert.equal(
						field['reason'],
						control.name === 'empty-target' ? 'empty_target' : 'unreadable',
					);
				}
			}
		});
		assert.equal(observer.captures.length, 0);
		writeJson(path.join(evidence, 'pass.json'), {
			target,
			corpus_revision: corpusRevision,
			real_requirements_found: 10,
			real_entities_found: 5,
			real_missing: [],
			real_fragment_hrefs: 0,
			prepared_real_asking: 1,
			prepared_labelled_asking: 1,
			labelled_controls: controls.flatMap((control) => control.expected),
			model_requests: 0,
			limits: [
				'Zero real fragment hrefs provide inventory only; labelled controls exercise anchors.',
				'No model quality, provider, release or platform acceptance.',
				'Markdown parity and native language/admission/budget joins are recorded separately.',
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
			help: { type: 'boolean' },
		},
	});
	if (values.help)
		process.stdout.write(
			'Observe HTML evidence on exact Git corpus and labelled controls.\n--binary PATH --source-root CHECKOUT --commit FULL_SHA --sha256 DIGEST --evidence NEW_DIRECTORY\nKeyless --check/--prepare only; no selections or model dispatch. Markdown parity is a separate companion entry.\n',
		);
	else await observeHtml(optionsSchema.parse(values));
}
