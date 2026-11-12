/** Actual repository-wide --since refusals retain offending paths and configured limits. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireDebugSignature } from '../rulebook/rulebook-amendment.ts';
import {
	arrayValue,
	objectValue,
	parseObject,
	readObject,
	runSequence,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import {
	requireHumanRefusal,
	requireJsonRefusal,
	requireSarifRefusal,
	type ExpectedRefusal,
} from './report-checks.ts';
import {
	assetFile,
	checkedFile,
	configFile,
	pngBytes,
	prepareRepository,
} from './scope-fixtures.ts';

const schema = z.object({
	binary: z.string(),
	'source-root': z.string(),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	evidence: z.string(),
	'expected-version': z
		.string()
		.regex(/^\d+\.\d+\.\d+$/)
		.default('0.11.1'),
});
interface ScopeControl {
	readonly name: string;
	readonly exit: number;
	readonly limits?: JsonObject;
	readonly expected: ExpectedRefusal;
	readonly change: 'binary' | 'file-bytes' | 'total-bytes' | 'files';
}
const controls: readonly ScopeControl[] = [
	{
		name: 'changed-binary',
		exit: 2,
		expected: { path: assetFile, reason: /UnsupportedEncoding|unsupported.*encoding/i },
		change: 'binary',
	},
	{
		name: 'file-bytes',
		exit: 3,
		limits: { max_file_bytes: 4096 },
		expected: {
			path: checkedFile,
			reason: /limit|file.?bytes/i,
			limit: { name: 'file_bytes', value: 4096 },
		},
		change: 'file-bytes',
	},
	{
		name: 'total-bytes',
		exit: 3,
		limits: { max_file_bytes: 65536, max_total_bytes: 16384 },
		expected: {
			path: checkedFile,
			reason: /limit|total.?bytes/i,
			limit: { name: 'total_bytes', value: 16384 },
		},
		change: 'total-bytes',
	},
	{
		name: 'file-count',
		exit: 3,
		limits: { max_files: 1 },
		expected: { path: assetFile, reason: /limit|files/i, limit: { name: 'files', value: 1 } },
		change: 'files',
	},
];
export async function observeScopeErrors(options: z.infer<typeof schema>): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	requireDebugSignature(target.binary);
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		const positive = path.join(evidence, 'equal-binary');
		const positiveBase = prepareRepository({
			root: positive,
			endpoint: observer.endpoint,
			binaryAsset: true,
		});
		filesystem.writeFileSync(
			path.join(positive, checkedFile),
			'BODY changed while binary stayed equal.\n',
		);
		await observer.run({
			root: positive,
			name: 'positive-check',
			arguments: ['lint', '--config', configFile, '--check'],
			expectedExit: 0,
		});
		const accepted = await observer.report({
			root: positive,
			name: 'equal-binary-positive',
			arguments: ['lint', '--config', configFile, '--since', positiveBase, '--format', 'json'],
			expectedExit: 0,
		});
		assert.equal(accepted.captures.length, 0);
		assert.equal(accepted.report['version'], options['expected-version']);
		const positiveRows = arrayValue(accepted.report['results']).map(objectValue);
		assert.equal(
			positiveRows.length,
			1,
			'Positive change scope dropped or widened the checked piece',
		);
		assert.equal(positiveRows[0]?.['file'], checkedFile);
		assert.equal(positiveRows[0]?.['outcome'], 'pass');
		await runSequence(controls, async (control) => {
			const root = path.join(evidence, control.name);
			const base = prepareRepository({
				root,
				endpoint: observer.endpoint,
				...(control.limits ? { limits: control.limits } : {}),
			});
			if (control.change === 'binary')
				filesystem.writeFileSync(path.join(root, assetFile), pngBytes);
			else
				filesystem.writeFileSync(
					path.join(root, checkedFile),
					control.change === 'file-bytes'
						? `BODY ${'x'.repeat(8192)}\n`
						: control.change === 'total-bytes'
							? `BODY ${'x'.repeat(32768)}\n`
							: 'BODY changed.\n',
				);
			writeJson(path.join(evidence, `${control.name}.input.json`), {
				base,
				changed_file: control.change === 'binary' ? assetFile : checkedFile,
				expected_refusal_path: control.expected.path,
				limits: control.limits ?? null,
			});
			await observer.run({
				root,
				name: `${control.name}-check`,
				arguments: ['lint', '--config', configFile, '--check'],
				expectedExit: 0,
			});
			await runSequence(['default', 'json', 'sarif'] as const, async (format) => {
				const saved = path.join(evidence, `${control.name}-${format}.report.json`);
				const result = await observer.run({
					root,
					name: `${control.name}-${format}`,
					arguments: [
						'lint',
						'--config',
						configFile,
						'--since',
						base,
						'--format',
						format,
						'--report',
						saved,
					],
					expectedExit: control.exit,
				});
				assert.equal(result.captures.length, 0);
				const report = readObject(saved);
				assert.equal(report['version'], options['expected-version']);
				assert.equal(objectValue(report['usage'])['model_requests'], 0);
				requireJsonRefusal(report, root, control.expected);
				if (format === 'default')
					requireHumanRefusal(
						Buffer.concat([result.stdout, result.stderr]).toString('utf8'),
						control.expected,
					);
				else if (format === 'json')
					requireJsonRefusal(parseObject(result.stdout.toString('utf8')), root, control.expected);
				else
					requireSarifRefusal(parseObject(result.stdout.toString('utf8')), root, control.expected);
			});
		});
		writeJson(path.join(evidence, 'pass.json'), {
			target,
			controls: controls.map((control) => ({
				name: control.name,
				expected_exit: control.exit,
				expected_path: control.expected.path,
				configured_limit: control.expected.limit ?? null,
			})),
			formats: ['default', 'json', 'sarif'],
			unchanged_binary_positive: true,
			http_requests: 0,
			limits: [
				'CLI encoding and three acquisition-limit stops only; deterministic SourceChanged and non-file/global limits require current native source-bound joins.',
				'No original user repository was modified.',
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
			'expected-version': { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (values.help)
		process.stdout.write(
			'Observe actual --since encoding and file/count/total-byte limit refusals in human, JSON and SARIF output.\n--binary PATH --source-root CHECKOUT --commit FULL_SHA --sha256 DIGEST --evidence NEW_DIRECTORY [--expected-version VERSION]\nKeeps unchanged binary positive, offending path fields and configured limit names/values. Keyless isolated repositories; owns bounded children/listener. SourceChanged causal proof is a separate native join.\n',
		);
	else await observeScopeErrors(schema.parse(values));
}
