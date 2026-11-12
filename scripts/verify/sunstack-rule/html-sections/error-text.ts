/** Observe real malformed HTML, exact released parity and safe-prefix cuts, without model dispatch. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireSignedArtifact } from '../git-first/capture-oracle.ts';
import {
	requireOutsideInput,
	requireSameFrozenInput,
	snapshotFrozenInput,
} from '../git-first/frozen-inputs.ts';
import {
	arrayValue,
	objectValue,
	readObject,
	textValue,
	writeJson,
	runSequence,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, releasedTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { errorControls, prepareErrorProject } from './error-fixtures.ts';
import {
	requireRecoveredControl,
	requireAmpersandAskingCoverage,
	reportRequests,
	requireVersionOnlyBytes,
} from './error-oracle.ts';
import { requireRecoveringParity, requireSourceWords } from './error-parity.ts';
import { askingRule, sectionFields } from './fixtures.ts';

const manifestSchema = z.object({
	ampersands: z.array(z.object({ file: z.string(), sha256: z.string() })).length(3),
	revision: z.string().regex(/^[a-f0-9]{40}$/),
	files: z.array(z.object({ file: z.string(), sha256: z.string() })),
	cases: z.array(
		z.object({
			id: z.string(),
			file: z.string(),
			lines: z.tuple([z.number(), z.number()]),
			text_sha256: z.string(),
		}),
	),
});
const ownSourceSchema = z.object({
	cases: z.array(z.object({ id: z.string(), raw_own_sha256: z.string(), source_text: z.string() })),
});
const optionsSchema = z.object({
	phase: z.enum(['baseline', 'candidate']),
	evidence: z.string(),
	'input-root': z.string(),
	'parity-root': z.string(),
	'baseline-binary': z.string(),
	'baseline-sha256': z.string().regex(/^[a-f0-9]{64}$/),
	binary: z.string().optional(),
	'source-root': z.string(),
	commit: z.string().optional(),
	sha256: z.string().optional(),
	'baseline-run': z.string().optional(),
	'shared-output': z.string(),
});

export async function observeErrorText(options: z.infer<typeof optionsSchema>): Promise<void> {
	const baseline = releasedTarget(options['baseline-binary'], options['baseline-sha256'], '0.14.0');
	const target =
		options.phase === 'baseline'
			? baseline
			: declaredTarget({
					binary: z.string().parse(options.binary),
					source: options['source-root'],
					commit: z.string().parse(options.commit),
					digest: z.string().parse(options.sha256),
				});
	requireSignedArtifact(target.binary);
	const evidence = path.resolve(options.evidence);
	const input = path.resolve(options['input-root']);
	const parity = path.resolve(options['parity-root']);
	const output = path.resolve(options['shared-output']);
	for (const root of [input, parity, options['source-root']]) {
		requireOutsideInput(root, evidence);
		requireOutsideInput(root, output);
	}
	filesystem.mkdirSync(evidence);
	filesystem.mkdirSync(output, { recursive: true });
	const manifest = manifestSchema.parse(
		readObject(path.join(import.meta.dirname, 'error-corpus.json')),
	);
	assert.equal(manifest.files.length, 13);
	assert.equal(manifest.cases.length, 43);
	function checkOriginals(): void {
		for (const entry of [...manifest.files, ...manifest.ampersands])
			assert.equal(
				createHash('sha256')
					.update(filesystem.readFileSync(path.join(input, entry.file)))
					.digest('hex'),
				entry.sha256,
			);
	}
	checkOriginals();
	const originalFixtures = arrayValue(
		JSON.parse(filesystem.readFileSync(path.join(parity, 'original-fixtures.json'), 'utf8')),
	).map(objectValue);
	function checkParityInputs(): void {
		for (const fixture of originalFixtures) {
			const root = textValue(fixture['root']);
			assert.equal(root, path.join(parity, textValue(fixture['name'])));
			for (const value of arrayValue(fixture['files'])) {
				const entry = objectValue(value);
				assert.equal(
					createHash('sha256')
						.update(filesystem.readFileSync(path.join(root, textValue(entry['file']))))
						.digest('hex'),
					entry['sha256'],
				);
			}
		}
	}
	checkParityInputs();
	writeJson(path.join(evidence, 'original-fixtures.json'), originalFixtures);
	const aiTools = path.join(parity, 'ai-tools');
	const frozen = snapshotFrozenInput(aiTools, 'HEAD', 'cde8d5cf5b9141a08264cc5d57e3e3a58f0f2aac');
	writeJson(path.join(evidence, 'input-before.json'), frozen);
	const files = Object.fromEntries(
		manifest.files.map((entry) => [
			entry.file,
			filesystem.readFileSync(path.join(input, entry.file), 'utf8'),
		]),
	);
	// A common project path and report path keep every serialized absolute path identical between subjects.
	const j1Root = path.join(output, 'j1');
	const controls = path.join(output, 'controls');
	const parser = path.join(output, 'parser');
	const html = path.join(output, 'html-sections');
	const amps = path.join(output, 'ampersands');
	const ampParser = path.join(output, 'amp-parser');
	if (options.phase === 'baseline') {
		prepareErrorProject(j1Root, files);
		const config = readObject(path.join(j1Root, 'sunstack.lint.json'));
		objectValue(config['evidence'])['kinds'] = {
			section: {
				html_units: { mode: 'unit', units: ['section'] },
				fields: { own: sectionFields.own, parent: sectionFields.parent },
				max_chars: 300_000,
			},
		};
		config['limits'] = { request_max_chars: 100_000 };
		writeJson(path.join(j1Root, 'sunstack.lint.json'), config);
		writeJson(path.join(j1Root, 'rules.json'), {
			K001: {
				...askingRule,
				questions: {
					quality: {
						type: 'noul',
						instructions: {
							check: 'own',
							context: ['parent'],
							task: 'Read the section explanation and its enclosing introduction.',
						},
					},
				},
			},
		});
		prepareErrorProject(
			controls,
			Object.fromEntries(errorControls.map((control) => [`${control.name}.html`, control.raw])),
		);
		prepareErrorProject(parser, { 'page-03.html': textValue(files['pages/page-03.html']) }, true);
		const ampFiles = Object.fromEntries(
			manifest.ampersands.map((entry) => [
				entry.file,
				filesystem.readFileSync(path.join(input, entry.file), 'utf8'),
			]),
		);
		prepareErrorProject(amps, ampFiles);
		prepareErrorProject(ampParser, ampFiles, true);
		const ampConfig = readObject(path.join(amps, 'sunstack.lint.json'));
		ampConfig['limits'] = {
			max_file_bytes: 16_777_216,
			request_max_chars: 100_000,
			max_input_tokens_per_request: 128_000,
		};
		const parserConfig = readObject(path.join(ampParser, 'sunstack.lint.json'));
		parserConfig['limits'] = { max_file_bytes: 16_777_216 };
		writeJson(path.join(ampParser, 'sunstack.lint.json'), parserConfig);
		filesystem.writeFileSync(
			path.join(amps, 'evidence/rules/html.yml'),
			"id: whole\nlanguage: Html\nrule:\n  any:\n    - {kind: element, regex: '^<(title|h[1-6])[ >]'}\n    - {kind: element, regex: '^<button id=\"tab-components\"'}\n",
		);
		writeJson(path.join(amps, 'sunstack.lint.json'), ampConfig);
		filesystem.cpSync(path.join(parity, 'html-sections'), html, { recursive: true });
		const htmlConfig = readObject(path.join(html, 'sunstack.lint.json'));
		htmlConfig['limits'] = { request_max_chars: 100_000, max_input_tokens_per_request: 128_000 };
		writeJson(path.join(html, 'sunstack.lint.json'), htmlConfig);
	}
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	const account: JsonObject[] = [];
	try {
		const rows = [
			{ name: 'j1', root: j1Root, arguments: ['lint', '--all'], exit: 6 },
			{ name: 'controls', root: controls, arguments: ['lint', '--all'], exit: 6 },
			{ name: 'ampersands', root: amps, arguments: ['lint', '--all'], exit: 6 },
			{
				name: 'html-sections',
				root: html,
				arguments: ['lint', '--all'],
				exit: 6,
			},
			{
				name: 'markdown-anchors',
				root: path.join(parity, 'anchors'),
				arguments: ['lint', '--all'],
				exit: 1,
			},
			{
				name: 'all-prepare',
				root: aiTools,
				arguments: [
					'lint',
					'--config',
					'plugins/skill-authoring/skills/domain-skill-crafting/rules/skill-rules.config.json',
					'--all',
				],
				exit: 6,
			},
		];
		await runSequence(rows, async (row) => {
			const disk = filesystem.statfsSync(evidence);
			assert.ok(disk.bavail * disk.bsize > 6 * 1024 ** 3);
			await observer.run({
				root: row.root,
				name: `${row.name}-check`,
				arguments: [
					...(row.name === 'all-prepare' ? row.arguments.slice(0, 3) : ['lint']),
					'--check',
				],
				expectedExit: 0,
			});
			const reportFile = path.join(output, `${row.name}.json`);
			if (filesystem.existsSync(reportFile)) filesystem.unlinkSync(reportFile);
			await observer.run({
				root: row.root,
				name: row.name,
				arguments: [...row.arguments, '--prepare', reportFile],
				expectedExit: row.exit,
			});
			const bytes = filesystem.readFileSync(reportFile);
			filesystem.writeFileSync(path.join(evidence, `${row.name}.json`), bytes);
			const report = readObject(reportFile);
			assert.equal(objectValue(report['usage'])['model_requests'], 0);
			const requests = arrayValue(report['requests']);
			if (row.name === 'all-prepare') assert.equal(requests.length, 499);
			if (row.name === 'html-sections') assert.equal(requests.length, 114);
			if (row.name === 'markdown-anchors') {
				assert.equal(requests.length, 0);
				assert.equal(arrayValue(report['findings']).length, 3);
			}
			if (row.name === 'j1') {
				const selected = [];
				for (const original of manifest.cases) {
					const lines = textValue(files[original.file]).split(/(?<=\n)/u);
					assert.equal(
						createHash('sha256')
							.update(lines.slice(original.lines[0] - 1, original.lines[1]).join(''))
							.digest('hex'),
						original.text_sha256,
						'Original J1 case-line fingerprint changed',
					);
					const matches = [...reportRequests(report).values()].filter(({ piece }) => {
						const where = objectValue(piece['where']);
						return (
							where['file'] === original.file &&
							where['first_line'] === original.lines[0] &&
							where['last_line'] === original.lines[1]
						);
					});
					assert.equal(
						matches.length,
						1,
						`J1 ${original.id} must join its exact original raw range`,
					);
					const match = matches[0];
					assert.ok(match);
					selected.push({ id: original.id, ...match });
				}
				const sourceCases = ownSourceSchema.parse(
					readObject(path.join(import.meta.dirname, 'error-source.json')),
				).cases;
				assert.equal(sourceCases.length, 43);
				const sourceAudit = selected.map((item) => {
					const sourceCase = sourceCases.find((entry) => entry.id === item.id);
					assert.ok(sourceCase);
					const raw = textValue(objectValue(objectValue(item.piece['fields'])['own'])['value']);
					assert.equal(createHash('sha256').update(raw).digest('hex'), sourceCase.raw_own_sha256);
					let missing = false;
					try {
						requireSourceWords(sourceCase.source_text, textValue(item.state['own']));
					} catch {
						missing = true;
					}
					return {
						id: item.id,
						missing_source_words: missing,
						source: sourceCase.source_text,
						asking: item.state['own'],
					};
				});
				writeJson(path.join(evidence, 'j1-source-audit.json'), { cases: sourceAudit });
				assert.deepEqual(
					sourceAudit.filter((item) => item.missing_source_words).map((item) => item.id),
					options.phase === 'baseline' ? ['j1-dev-006'] : [],
				);
				writeJson(path.join(evidence, 'j1-selected.json'), {
					revision: manifest.revision,
					cases: selected,
				});
			}
			if (options.phase === 'candidate') {
				const prior = path.join(z.string().parse(options['baseline-run']), `${row.name}.json`);
				if (
					row.name === 'all-prepare' ||
					row.name === 'markdown-anchors' ||
					row.name === 'ampersands'
				)
					requireVersionOnlyBytes(filesystem.readFileSync(prior), bytes);
				if (row.name === 'j1' || row.name === 'html-sections')
					writeJson(path.join(evidence, `${row.name}-exact-diff.json`), {
						changes: requireRecoveringParity(
							filesystem.readFileSync(prior),
							bytes,
							row.name === 'j1',
						),
					});
				if (row.name === 'controls')
					requireRecoveredControl(
						report,
						Object.fromEntries(errorControls.map((control) => [`${control.name}.html`, control])),
					);
			}
			account.push({
				name: row.name,
				requests: requests.length,
				bytes: bytes.length,
				exit: row.exit,
			});
			writeJson(path.join(evidence, 'account.json'), account);
		});
		await observer.run({
			root: parser,
			name: 'parser-check',
			arguments: ['lint', '--check'],
			expectedExit: 0,
		});
		await observer.run({
			root: parser,
			name: 'parser-errors',
			arguments: ['lint', '--all', '--evidence', 'section'],
			expectedExit: 0,
		});
		await observer.run({
			root: ampParser,
			name: 'amp-parser-check',
			arguments: ['lint', '--check'],
			expectedExit: 0,
		});
		const ampErrors = await observer.run({
			root: ampParser,
			name: 'amp-parser-errors',
			arguments: ['lint', '--all', '--evidence', 'section'],
			expectedExit: 0,
		});
		writeJson(path.join(evidence, 'ampersand-asking-coverage.json'), {
			error_records: requireAmpersandAskingCoverage(
				readObject(path.join(evidence, 'ampersands.json')),
				objectValue(JSON.parse(ampErrors.stdout.toString('utf8'))),
			),
		});
		if (options.phase === 'candidate')
			requireVersionOnlyBytes(
				filesystem.readFileSync(
					path.join(z.string().parse(options['baseline-run']), 'run/amp-parser-errors/stdout.log'),
				),
				ampErrors.stdout,
			);
		assert.equal(observer.captures.length, 0);
	} finally {
		try {
			await observer.close();
		} finally {
			checkOriginals();
			checkParityInputs();
			const after = snapshotFrozenInput(aiTools, 'HEAD', frozen.head);
			writeJson(path.join(evidence, 'input-after.json'), after);
			requireSameFrozenInput(frozen, after);
		}
	}
	writeJson(path.join(evidence, 'captured.json'), {
		target,
		phase: options.phase,
		account,
		owned_http_requests: 0,
		originals_unchanged: true,
		outstanding: [
			'Inspect real raw/projected J1 diff and wrapped-page bounded exception before final handback.',
		],
	});
}

if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: {
			...Object.fromEntries(
				Object.keys(optionsSchema.shape).map((name) => [name, { type: 'string' as const }]),
			),
			help: { type: 'boolean' },
		},
	});
	if (values.help)
		process.stdout.write(
			'Observe malformed HTML using existing keyless CLI harness. --phase baseline|candidate --baseline-binary PATH --baseline-sha256 SHA --source-root CHECKOUT --input-root FROZEN_J1 --parity-root ORIGINAL_INPUTS --shared-output COMMON_OUTSIDE_DIRECTORY --evidence NEW_DIRECTORY [--binary PATH --commit FULL_HEAD --sha256 SHA --baseline-run BASELINE_EVIDENCE]. Expected source:0.14.1 commission C1–C7. Capture does not itself close J1/raw/parity review.\n',
		);
	else await observeErrorText(optionsSchema.parse(values));
}
