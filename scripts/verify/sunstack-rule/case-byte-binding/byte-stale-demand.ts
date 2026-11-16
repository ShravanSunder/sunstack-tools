/** Model-backed selection demand: stale cases must not dispatch a child or extra requests. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	gitCommand,
	objectValue,
	parseObject,
	readObject,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import { responseBody, type RulebookProcess } from '../rulebook/rulebook-process.ts';
import { byteFixtureCases, byteFixturePath, createByteFixture } from './byte-fixture.ts';

function orderedCaptures(captures: readonly string[]): string[] {
	return [...captures].toSorted((left, right) => left.localeCompare(right));
}

export async function observeStaleDemand(options: {
	readonly observer: RulebookProcess;
	readonly evidence: string;
}): Promise<void> {
	const root = path.join(options.evidence, 'stale-demand-fixture');
	const fixture = createByteFixture(root, options.observer.endpoint);
	const text = '<p>café yes</p>';
	filesystem.writeFileSync(path.join(root, byteFixturePath), text);
	filesystem.writeFileSync(path.join(root, 'docs', 'stale.html'), text);
	const configPath = path.join(root, 'sunstack.lint.json');
	const config = readObject(configPath);
	const evidence = objectValue(config['evidence']);
	const kinds = objectValue(evidence['kinds']);
	const paragraph = objectValue(kinds['paragraph']);
	paragraph['select'] = {
		question: { type: 'noul', instructions: { check: 'body', task: 'Select this paragraph.' } },
		yes_at_or_above: 0.8,
		no_at_or_below: 0.2,
	};
	writeJson(configPath, config);
	gitCommand(root, ['add', byteFixturePath, 'docs/stale.html', 'sunstack.lint.json']);
	gitCommand(root, ['commit', '-qm', 'Model selection and stale file']);
	const revision = gitCommand(root, ['rev-parse', 'HEAD']);
	const common = {
		...byteFixtureCases[0],
		kind: 'rule',
		rule: 'approval',
		revision,
		lines: [1, 1],
		set: 'development',
		label_from: 'blind-agents',
	};
	const valid = { ...common, id: 'valid', file: byteFixturePath };
	const stale = { ...common, id: 'stale', file: 'docs/stale.html', text_sha256: '0'.repeat(64) };
	writeJson(fixture.casesPath, { repository: root, cases: [valid, stale], bars: [] });
	const originalResponder = options.observer.respond;
	options.observer.respond = async ({ body }) => ({ status: 200, body: responseBody(body, 0.91) });
	try {
		const run = async (
			name: string,
			selected: readonly string[],
		): Promise<{ report: ReturnType<typeof parseObject>; captures: string[] }> => {
			const reportPath = path.join(options.evidence, `stale-${name}.json`);
			const selectedArgs = selected.flatMap((id) => ['--case', id]);
			const result = await options.observer.run({
				root,
				name: `stale-${name}`,
				arguments: [
					'lint',
					'--config',
					'sunstack.lint.json',
					'--report',
					reportPath,
					'cases',
					fixture.casesPath,
					'--set',
					'development',
					...selectedArgs,
				],
				credential: true,
				expectedExit: 1,
			});
			return {
				report: parseObject(filesystem.readFileSync(reportPath, 'utf8')),
				captures: result.captures.map((capture) => capture.raw.toString('utf8')),
			};
		};
		const positive = await run('positive', ['valid']);
		assert.ok(
			positive.captures.length >= 2,
			'Positive did not witness selection and rule requests',
		);
		const runReport = objectValue(objectValue(arrayValue(positive.report['runs'])[0])['report']);
		const owners = new Set(
			arrayValue(runReport['requests']).map(
				(request) => objectValue(objectValue(request)['owner'])['kind'],
			),
		);
		assert.ok(
			owners.has('selection') && owners.has('rule'),
			'Positive lacks both selection and rule owners',
		);
		const only = await run('only', ['stale']);
		assert.equal(only.captures.length, 0);
		assert.equal(arrayValue(only.report['runs']).length, 0, 'Stale-only case spawned a child run');
		const onlyRows = arrayValue(only.report['cases']).map(objectValue);
		assert.equal(onlyRows.length, 1);
		assert.equal(objectValue(onlyRows[0]?.['outcome'])['kind'], 'stale');
		const mixed = await run('mixed', ['valid', 'stale']);
		assert.deepEqual(
			orderedCaptures(mixed.captures),
			orderedCaptures(positive.captures),
			'Stale file caused extra or changed requests',
		);
		const rows = arrayValue(mixed.report['cases']).map(objectValue);
		assert.deepEqual(
			rows.map((row) => [row['id'], objectValue(row['outcome'])['kind']]),
			[
				['valid', 'right'],
				['stale', 'stale'],
			],
		);
	} finally {
		options.observer.respond = originalResponder;
	}
}
