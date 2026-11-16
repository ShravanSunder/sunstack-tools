/** Compare successful checked-file acquisition with an excluded regular-file binding. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import {
	arrayValue,
	objectValue,
	parseObject,
	readObject,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { requireSdk } from '../sdk/sdk-target.ts';
import { createByteFixture } from './byte-fixture.ts';

function binding(id: string, file: string, content: string): object {
	return {
		id,
		kind: 'rule',
		rule: 'approval',
		file,
		lines: [1, 1],
		text_sha256: createHash('sha256').update(content).digest('hex'),
	};
}

async function main(): Promise<void> {
	const { values } = parseArgs({
		strict: true,
		options: {
			binary: { type: 'string' },
			source: { type: 'string' },
			commit: { type: 'string' },
			sha256: { type: 'string' },
			evidence: { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (values.help) {
		process.stdout.write(
			'Baseline acquisition-count capability probe: --binary <absolute CLI> --source <absolute source> --commit <full SHA> --sha256 <binary hash> --evidence <new absolute directory>. Uses readable sentinel positive and comparable selected demand. Counts detect successful checked reads, not failed read attempts. Baseline line bindings only.\n',
		);
		return;
	}
	requireSdk(
		values.binary &&
			path.isAbsolute(values.binary) &&
			values.source &&
			path.isAbsolute(values.source),
		'Absolute binary and source required',
	);
	requireSdk(
		values.commit && values.sha256 && values.evidence && path.isAbsolute(values.evidence),
		'Commit, digest and new evidence required',
	);
	const evidence = values.evidence;
	const target = declaredTarget({
		binary: values.binary,
		source: values.source,
		commit: values.commit,
		digest: values.sha256,
	});
	filesystem.mkdirSync(values.evidence, { recursive: false });
	const observer = new RulebookProcess(target, path.join(values.evidence, 'cli'));
	let started = false;
	try {
		await observer.start();
		started = true;
		const root = path.join(values.evidence, 'fixture');
		createByteFixture(root, observer.endpoint);
		const scopeText = '<p>Selected scope.</p>\n';
		const sentinelText = '<p>Readable sentinel.</p>\n';
		filesystem.writeFileSync(path.join(root, 'docs', 'pieces.html'), scopeText);
		filesystem.writeFileSync(path.join(root, 'docs', 'sentinel.html'), sentinelText);
		const configPath = path.join(root, 'sunstack.lint.json');
		const config = readObject(configPath);
		const scopeBinding = binding('scope', 'docs/pieces.html', scopeText);
		const sentinelBinding = binding('sentinel', 'docs/sentinel.html', sentinelText);
		const run = async (
			name: string,
			bindings: readonly object[],
		): Promise<{ count: unknown; captures: string[]; report: ReturnType<typeof parseObject> }> => {
			const inputPath = path.join(evidence, `${name}.json`);
			writeJson(inputPath, bindings);
			const result = await observer.run({
				root,
				name,
				arguments: [
					'lint',
					'--config',
					'sunstack.lint.json',
					'--all',
					'--format',
					'json',
					'--case-selection',
					inputPath,
				],
				credential: true,
				expectedExit: 0,
			});
			const report = parseObject(result.stdout.toString('utf8'));
			return {
				count: objectValue(report['usage'])['checked_files_acquired'],
				captures: result.captures.map((capture) => capture.raw.toString('utf8')),
				report,
			};
		};
		config['check'] = { files: ['docs/sentinel.html'] };
		writeJson(configPath, config);
		const sentinelPositive = await run('sentinel-positive', [sentinelBinding]);
		assert.equal(sentinelPositive.count, 1);
		assert.equal(sentinelPositive.captures.length, 1);
		config['check'] = { files: ['docs/pieces.html'] };
		writeJson(configPath, config);
		const baseline = await run('scope-positive', [scopeBinding]);
		assert.equal(baseline.count, 1);
		assert.equal(baseline.captures.length, 1);
		const mixed = await run('scope-plus-excluded', [scopeBinding, sentinelBinding]);
		assert.equal(
			mixed.count,
			baseline.count,
			'Excluded binding increased successfully acquired file count',
		);
		assert.deepEqual(
			mixed.captures,
			baseline.captures,
			'Excluded binding changed controlled requests',
		);
		const rows = arrayValue(objectValue(mixed.report['case_selection'])['bindings']).map(
			objectValue,
		);
		assert.deepEqual(
			rows.map((row) => [objectValue(row['binding'])['id'], objectValue(row['outcome'])['kind']]),
			[
				['scope', 'selected'],
				['sentinel', 'unmatched'],
			],
		);
		writeJson(path.join(values.evidence, 'counts.json'), {
			sentinel_positive: sentinelPositive.count,
			scope_positive: baseline.count,
			scope_plus_excluded: mixed.count,
			request_counts: [
				sentinelPositive.captures.length,
				baseline.captures.length,
				mixed.captures.length,
			],
			limits: 'successful checked-file acquisitions only; not failed read attempts',
		});
	} finally {
		if (started) await observer.close();
	}
}
await main();
