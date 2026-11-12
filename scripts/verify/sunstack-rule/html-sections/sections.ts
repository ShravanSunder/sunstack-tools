/** Real lexical HTML outlines, complete asking text and truthful cut indicators via signed CLI. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireOutsideInput } from '../git-first/frozen-inputs.ts';
import { preparedPiece } from '../html-evidence/html-checks.ts';
import { writeProject } from '../html-evidence/html-fixtures.ts';
import { requireDebugSignature } from '../rulebook/rulebook-amendment.ts';
import { listingPieces } from '../rulebook/rulebook-evidence-checks.ts';
import {
	arrayValue,
	objectValue,
	numberValue,
	readObject,
	runSequence,
	writeJson,
	textValue,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { observeOtherAskingPaths } from './asking-paths.ts';
import { requireSections, requireCut } from './checks.ts';
import { corpus, requireCorpus } from './corpus.ts';
import {
	askingRule,
	edgeHtml,
	edgeSections,
	prepareProject,
	projectionHtml,
	projectionText,
} from './fixtures.ts';

const optionsSchema = z.object({
	binary: z.string(),
	'source-root': z.string(),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	'input-root': z.string(),
	evidence: z.string(),
});
export async function observeSections(options: z.infer<typeof optionsSchema>): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	requireDebugSignature(target.binary);
	const input = path.resolve(options['input-root']);
	requireCorpus(input);
	const originalModes = corpus.map(
		(entry) => filesystem.lstatSync(path.join(input, entry.file)).mode,
	);
	const evidence = path.resolve(options.evidence);
	requireOutsideInput(input, evidence);
	requireOutsideInput(target.source, evidence);
	filesystem.mkdirSync(evidence);
	writeJson(path.join(evidence, 'input-before.json'), {
		files: corpus.map((entry) => ({ file: entry.file, sha256: entry.sha256 })),
		modes: originalModes,
	});
	const real = path.join(evidence, 'real-pages');
	prepareProject({
		root: real,
		files: Object.fromEntries(
			corpus.map((entry) => [
				entry.file,
				filesystem.readFileSync(path.join(input, entry.file), 'utf8'),
			]),
		),
	});
	const edge = path.join(evidence, 'outline-controls');
	prepareProject({ root: edge, files: { 'edge.html': edgeHtml } });
	const cut = path.join(evidence, 'cut-controls');
	prepareProject({ root: cut, files: { 'edge.html': edgeHtml }, maxChars: 1200 });
	const byteBudget = path.join(evidence, 'byte-budget');
	prepareProject({ root: byteBudget, files: { 'edge.html': edgeHtml } });
	const byteConfig = readObject(path.join(byteBudget, 'sunstack.lint.json'));
	byteConfig['limits'] = { max_file_bytes: 512, max_total_bytes: 512 };
	writeJson(path.join(byteBudget, 'sunstack.lint.json'), byteConfig);
	const legacy = path.join(evidence, 'legacy-html-asking');
	filesystem.mkdirSync(legacy);
	filesystem.writeFileSync(path.join(legacy, 'projection.html'), projectionHtml);
	writeProject(
		legacy,
		{
			K001: {
				...askingRule,
				questions: {
					quality: {
						type: 'noul',
						instructions: { check: 'full', task: 'Inspect the complete picture text.' },
					},
				},
			},
		},
		{ section: { rule: 'whole', fields: { full: { from: 'remainder' } } } },
		['projection.html'],
		'id: whole\nlanguage: Html\nrule: {kind: document}\n',
	);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	let pass: JsonObject | undefined;
	try {
		await runSequence([real, edge, cut, byteBudget, legacy], async (root) => {
			const config = readObject(path.join(root, 'sunstack.lint.json'));
			objectValue(config['decision_model'])['endpoint'] = observer.endpoint;
			writeJson(path.join(root, 'sunstack.lint.json'), config);
			await observer.run({
				root,
				name: `check-${path.basename(root)}`,
				arguments: ['lint', '--check'],
				expectedExit: 0,
			});
		});
		const listing = await observer.run({
			root: real,
			name: 'real-outline',
			arguments: ['lint', '--all', '--evidence', 'section'],
			expectedExit: 0,
		});
		const pieces = listingPieces(listing.stdout);
		assert.equal(
			pieces.length,
			corpus.reduce((total, entry) => total + entry.sections.length, 0),
		);
		for (const entry of corpus)
			requireSections({
				pieces: pieces.filter((piece) => objectValue(piece['where'])['file'] === entry.file),
				sections: entry.sections,
				raw: filesystem.readFileSync(path.join(real, entry.file)),
			});
		const controls = await observer.run({
			root: edge,
			name: 'edge-outline',
			arguments: ['lint', '--all', '--evidence', 'section'],
			expectedExit: 0,
		});
		requireSections({
			pieces: listingPieces(controls.stdout),
			sections: edgeSections(),
			raw: Buffer.from(edgeHtml),
		});
		const prepare = async (root: string, name: string): Promise<JsonObject> => {
			const file = path.join(evidence, `${name}.json`);
			await observer.run({
				root,
				name,
				arguments: ['lint', '--all', '--prepare', file],
				expectedExit: 6,
			});
			const report = readObject(file);
			assert.equal(objectValue(report['usage'])['model_requests'], 0);
			assert.equal(arrayValue(report['results']).length, 0);
			return report;
		};
		const prepared = await prepare(edge, 'edge-prepared');
		const requests = arrayValue(prepared['requests']).map(objectValue);
		assert.equal(
			requests.length,
			7,
			'Every outline section, including top level, must prepare normally',
		);
		const starts = new Set<number>();
		for (const request of requests) {
			assert.equal(request['status'], 'prepared');
			const piece = preparedPiece(prepared, request);
			const start = numberValue(objectValue(objectValue(piece['where'])['span'])['byte_start']);
			assert.ok(!starts.has(start));
			starts.add(start);
			const expected = edgeSections().find((section) => section.start === start);
			assert.ok(expected);
			const state = objectValue(objectValue(request['request'])['state']);
			assert.deepEqual(state['unavailable-fields'], {});
			if (expected.parent === null)
				assert.equal(state['parent'], null, 'Top-level absence must remain ordinary judged input');
			if (expected.heading === 'parent') {
				assert.ok(
					textValue(objectValue(objectValue(piece['fields'])['own'])['value']).includes('Block 7:'),
				);
				assert.ok(
					textValue(state['own']).includes('[label: Late diagram]'),
					'Diagram past five blocks disappeared',
				);
			}
			if (expected.heading === 'child') {
				assert.ok(
					textValue(state['parent']).includes('[label: Late diagram]'),
					'Shared parent picture did not reach child',
				);
				assert.ok(!textValue(state['own']).includes('Late diagram'));
			}
			if (expected.heading === 'root') {
				assert.ok(textValue(state['own']).includes('[alt: Root image]'));
				assert.ok(textValue(state['own']).includes('[aria-label: Root accessible]'));
			}
		}
		const bounded = await prepare(cut, 'cut-prepared');
		assert.equal(arrayValue(bounded['requests']).length, 7);
		let truncated = 0;
		for (const request of arrayValue(bounded['requests']).map(objectValue)) {
			const piece = preparedPiece(bounded, request);
			const state = objectValue(objectValue(request['request'])['state']);
			const view = objectValue(piece['model_view']);
			assert.equal(view['state'], 'bounded');
			for (const field of arrayValue(view['cut']).map(textValue)) {
				requireCut({ piece, state, field });
				truncated++;
			}
		}
		assert.ok(truncated > 0, 'Budget fixture never exercised a cut');
		const exhausted = path.join(evidence, 'byte-budget.json');
		await observer.run({
			root: byteBudget,
			name: 'byte-budget-refusal',
			arguments: ['lint', '--all', '--prepare', exhausted],
			expectedExit: 3,
		});
		assert.equal(
			arrayValue(readObject(exhausted)['requests']).length,
			0,
			'Unread byte-budget input must not prepare a false no-picture judgment',
		);
		const projected = await prepare(legacy, 'legacy-projected');
		const projectionRequests = arrayValue(projected['requests']).map(objectValue);
		assert.equal(projectionRequests.length, 1);
		const projectionRequest = projectionRequests[0];
		assert.ok(projectionRequest);
		assert.equal(
			objectValue(objectValue(projectionRequest['request'])['state'])['full'],
			projectionText,
		);
		const projectionPiece = preparedPiece(projected, projectionRequest);
		assert.equal(projectionPiece['text'], projectionHtml);
		assert.equal(
			objectValue(objectValue(projectionPiece['fields'])['full'])['value'],
			projectionHtml,
			'Mechanical input lost raw HTML',
		);
		await observeOtherAskingPaths({ observer, evidence });
		assert.equal(observer.captures.length, 0);
		pass = {
			target: { ...target },
			real_counts: corpus.map((entry) => ({
				file: entry.file,
				sections: entry.sections.length,
				sha256: entry.sha256,
			})),
			controls: 7,
			truncated_fields: truncated,
			late_diagram: true,
			shared_parent: true,
			legacy_projection: projectionText,
			asking_paths: ['rule', 'selection candidate', 'selection shared', 'filter'],
			http_requests: 0,
			native_joins: [
				'Low-level selector uses the same outline owner; admission/filter rejections.',
				'Markdown parity and legacy HTML projection-only differences.',
				'Existing mechanical tools receive original HTML; byte-budget refusal is not partial evidence.',
			],
			limits: [
				'No provider/model-quality/CI/release acceptance.',
				'Original real-page bytes come from the fixed corpus pins; wrapped page is an approved untracked snapshot.',
			],
		};
	} finally {
		try {
			await observer.close();
		} finally {
			requireCorpus(input);
			const afterModes = corpus.map(
				(entry) => filesystem.lstatSync(path.join(input, entry.file)).mode,
			);
			writeJson(path.join(evidence, 'input-after.json'), {
				files: corpus.map((entry) => ({ file: entry.file, sha256: entry.sha256 })),
				modes: afterModes,
			});
			assert.deepEqual(afterModes, originalModes, 'Original corpus permissions changed');
		}
	}
	assert.ok(pass);
	writeJson(path.join(evidence, 'pass.json'), pass);
}
if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: {
			...Object.fromEntries(
				['binary', 'source-root', 'commit', 'sha256', 'input-root', 'evidence'].map((name) => [
					name,
					{ type: 'string' as const },
				]),
			),
			help: { type: 'boolean' },
		},
	});
	if (values.help)
		process.stdout.write(
			'Observe HTML heading outlines on three byte-pinned real pages plus owned edge/projection/cut controls.\n--binary PATH --source-root CHECKOUT --commit FULL_SHA --sha256 DIGEST --input-root FROZEN_CORPUS --evidence NEW_DIRECTORY\nInputs: flat.html, wrapped.html, mixed.html with corpus-*.json hashes. Keyless check/evidence/prepare only; no model dispatch.\n',
		);
	else await observeSections(optionsSchema.parse(values));
}
