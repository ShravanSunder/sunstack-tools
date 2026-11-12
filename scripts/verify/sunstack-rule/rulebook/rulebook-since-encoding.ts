/** Historical0.10.1/0.10.0 --since replay; current capture uses git-first/git-first-edges.ts. */
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	arrayValue,
	baselineGit,
	equal,
	gitCommand,
	objectValue,
	parseObject,
	readObject,
	repository,
	requireCondition,
	runSequence,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { requireComparableBytes } from './rulebook-output-comparison.ts';
import { declaredTarget, releasedTarget, RulebookProcess } from './rulebook-process.ts';

const pngSignature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
const optionsSchema = z.object({
	binary: z.string(),
	'source-root': z.string().default(repository),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	'baseline-binary': z.string(),
	'baseline-sha256': z.string().regex(/^[a-f0-9]{64}$/),
	evidence: z.string(),
});

function fixture(root: string, binaryAsset: boolean, endpoint: string): string {
	filesystem.mkdirSync(root);
	filesystem.mkdirSync(path.join(root, 'docs'));
	filesystem.mkdirSync(path.join(root, 'assets'));
	filesystem.mkdirSync(path.join(root, 'evidence/rules'), { recursive: true });
	filesystem.writeFileSync(path.join(root, 'evidence/sgconfig.yml'), 'ruleDirs: [rules]\n');
	filesystem.writeFileSync(
		path.join(root, 'evidence/rules/pieces.yml'),
		'id: document\nlanguage: Markdown\nrule:\n  kind: document\n',
	);
	filesystem.writeFileSync(
		path.join(root, 'rulebook.md'),
		'Keep the document short. <!-- K001 -->\n',
	);
	writeJson(path.join(root, 'rules.json'), {
		K001: {
			name: 'short',
			evidence: 'doc',
			severity: 'error',
			decision: {
				start: 'check',
				steps: {
					check: {
						mechanical_check: { tool: 'max_chars', field: 'body', max: 100 },
						yes: 'pass',
						['no']: { finding: 'Document is too long.' },
					},
				},
			},
		},
	});
	writeJson(path.join(root, 'sunstack.lint.json'), {
		rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
		marker_prefix: 'K',
		check: { files: ['docs/*.md'] },
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: { doc: { rule: 'document', fields: { body: { from: 'remainder' } } } },
		},
		decision_model: { model: 'jev-latest', endpoint },
	});
	filesystem.writeFileSync(path.join(root, 'docs/one.md'), 'Before.\n');
	filesystem.writeFileSync(path.join(root, 'docs/unchanged.md'), 'Unchanged peer.\n');
	filesystem.writeFileSync(
		path.join(root, 'assets/image.png'),
		binaryAsset ? pngSignature : Buffer.from('Plain UTF-8 asset.\n'),
	);
	baselineGit(root);
	const base = gitCommand(root, ['rev-parse', 'HEAD']);
	filesystem.writeFileSync(path.join(root, 'docs/one.md'), 'After.\n');
	return base;
}

function requireChangedPiece(report: JsonObject): void {
	const rows = arrayValue(report['results']).map(objectValue);
	equal(rows.length, 1, 'Scope did not retain exactly one changed checked piece');
	equal(rows[0]?.['file'], 'docs/one.md', 'Scope selected an unchanged checked peer');
	equal(rows[0]?.['outcome'], 'pass', 'Mechanical check did not execute');
	equal(objectValue(report['scope'])['widened'], null, 'Fixture unexpectedly widened scope');
	equal(objectValue(report['counts']), { error: 0, warn: 0 }, 'Fixture findings changed');
}

export async function main(): Promise<void> {
	const { values } = parseArgs({
		options: {
			help: { type: 'boolean' },
			binary: { type: 'string' },
			'source-root': { type: 'string' },
			commit: { type: 'string' },
			sha256: { type: 'string' },
			'baseline-binary': { type: 'string' },
			'baseline-sha256': { type: 'string' },
			evidence: { type: 'string' },
		},
	});
	if (values.help) {
		process.stdout.write(
			'Historical0.10.1 replay only; current route: git-first/git-first-edges.ts.\nUsage: node rulebook-since-encoding.ts --binary <candidate> --source-root <checkout> --commit <full-head> --sha256 <digest> --baseline-binary <0.10.0> --baseline-sha256 <digest> --evidence <new-directory>\nProves keyless real Git --since selection beside an equal PNG, changed binary refusal, and exact binary-free JSON/report/prepare parity apart from the root engine version. The comparator must report0.10.0; preserve its provenance separately. Uses no inference or provider credential; owns bounded CLI children and a zero-request loopback witness. Original affected repository and exact original PNG are separate commission-specific probes.\n',
		);
		return;
	}
	const options = optionsSchema.parse(values);
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	const baseline = releasedTarget(options['baseline-binary'], options['baseline-sha256'], '0.10.0');
	requireDebugSignature(target.binary);
	const version = spawnSync(target.binary, ['--version'], { encoding: 'utf8', timeout: 10000 });
	equal(version.status, 0, 'Candidate version probe failed');
	const candidateVersion = z
		.string()
		.regex(/^sunstack \d+\.\d+\.\d+$/)
		.parse(version.stdout.trim())
		.slice('sunstack '.length);
	equal(
		candidateVersion,
		'0.10.1',
		'Historical encoding replay requires0.10.1; use git-first-edges.ts for current capture',
	);
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const root = path.join(evidence, 'binary-fixture');
	const plain = path.join(evidence, 'plain-fixture');
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		const base = fixture(root, true, observer.endpoint);
		const plainBase = fixture(plain, false, observer.endpoint);
		await observer.run({ root, name: 'check', arguments: ['lint', '--check'], expectedExit: 0 });
		const old = await observer.run({
			root,
			target: baseline,
			name: 'baseline-binary',
			arguments: ['lint', '--since', base, '--format', 'json'],
			expectedExit: 2,
		});
		requireCondition(
			old.stdout.toString('utf8').includes('UnsupportedEncoding'),
			'Comparator failed before the intended encoding boundary',
		);
		const current = await observer.run({
			root,
			name: 'candidate-binary',
			arguments: ['lint', '--since', base, '--format', 'json'],
			expectedExit: 0,
		});
		requireChangedPiece(parseObject(current.stdout.toString('utf8')));
		const preparedPath = path.join(evidence, 'binary-prepared.json');
		await observer.run({
			root,
			name: 'candidate-binary-prepare',
			arguments: ['lint', '--since', base, '--prepare', preparedPath],
			expectedExit: 6,
		});
		const prepared = readObject(preparedPath);
		requireChangedPiece(prepared);
		equal(
			arrayValue(prepared['requests']).length,
			0,
			'Mechanical-only prepare invented a model request',
		);
		await runSequence(
			[
				{ name: 'selected-invalid-after', file: 'docs/one.md', bytes: Buffer.from([0xff]) },
				{
					name: 'invalid-asset-before',
					file: 'assets/image.png',
					bytes: Buffer.from('Now UTF-8.\n'),
				},
				{
					name: 'invalid-asset-after',
					file: 'assets/image.png',
					bytes: Buffer.concat([pngSignature, Buffer.from([0xff])]),
				},
			],
			async (control) => {
				const file = path.join(root, control.file);
				const original = filesystem.readFileSync(file);
				try {
					filesystem.writeFileSync(file, control.bytes);
					const refused = await observer.run({
						root,
						name: control.name,
						arguments: [
							'lint',
							'--since',
							base,
							'--prepare',
							path.join(evidence, `${control.name}.json`),
						],
						expectedExit: 2,
					});
					requireCondition(
						refused.stdout.toString('utf8').includes('UnsupportedEncoding'),
						'Changed binary was ignored or failed for another reason',
					);
				} finally {
					filesystem.writeFileSync(file, original);
				}
			},
		);
		await runSequence(['ordinary', 'prepare'] as const, async (mode) => {
			const beforePath = path.join(evidence, `plain-baseline-${mode}.json`);
			const afterPath = path.join(evidence, `plain-candidate-${mode}.json`);
			const common = ['lint', '--since', plainBase, '--format', 'json'];
			const flag = mode === 'prepare' ? '--prepare' : '--report';
			const before = await observer.run({
				root: plain,
				target: baseline,
				name: `plain-baseline-${mode}`,
				arguments: [...common, flag, beforePath],
				expectedExit: mode === 'prepare' ? 6 : 0,
			});
			const after = await observer.run({
				root: plain,
				name: `plain-candidate-${mode}`,
				arguments: [...common, flag, afterPath],
				expectedExit: mode === 'prepare' ? 6 : 0,
			});
			const compare = { baselineVersion: '0.10.0', candidateVersion };
			requireComparableBytes({
				...compare,
				before: before.stdout,
				after: after.stdout,
				format: 'json',
			});
			requireComparableBytes({
				...compare,
				before: filesystem.readFileSync(beforePath),
				after: filesystem.readFileSync(afterPath),
				format: 'report',
			});
			requireChangedPiece(readObject(afterPath));
		});
		equal(observer.captures.length, 0, 'Keyless encoding journey dispatched a request');
		writeJson(path.join(evidence, 'pass.json'), {
			commit: target.commit,
			binary_sha256: target.digest,
			candidate_version: candidateVersion,
			baseline,
			changed_piece: 'docs/one.md',
			unchanged_binary: 'assets/image.png',
			refusal_controls: ['selected-invalid-after', 'invalid-asset-before', 'invalid-asset-after'],
			binary_free_comparisons: ['ordinary JSON/report', 'prepare JSON/report'],
			version_exceptions: ['root version only'],
			http_requests: 0,
			native_joins_required: [
				'equal-binary snapshot mutation',
				'file/file-byte/total-byte limits',
				'present-empty add/delete',
				'identical-blob index/commit',
			],
			not_covered: [
				'real affected repository',
				'exact original PNG',
				'provider inference',
				'release acceptance',
			],
		});
	} finally {
		await observer.close();
	}
	process.stdout.write(
		'Equal-binary --since selection, changed-encoding refusal and binary-free parity verified.\n',
	);
}
if (process.argv[1] && path.resolve(process.argv[1]) === import.meta.filename) await main();
