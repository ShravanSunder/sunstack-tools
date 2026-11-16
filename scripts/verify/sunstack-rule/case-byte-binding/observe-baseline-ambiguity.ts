/** Probe genuine same-range HTML document and element pieces on a declared baseline. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { arrayValue, objectValue, parseObject } from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { requireSdk, saveSdkEvidence } from '../sdk/sdk-target.ts';
import { createByteFixture } from './byte-fixture.ts';

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
			'Baseline capability probe: --binary <absolute CLI> --source <absolute source root> --commit <full SHA> --sha256 <binary hash> --evidence <new absolute directory>. Requires a frozen declared baseline; it does not prove E5 byte selection.\n',
		);
		return;
	}
	requireSdk(values.binary && path.isAbsolute(values.binary), 'Absolute binary required');
	requireSdk(values.source && path.isAbsolute(values.source), 'Absolute source required');
	requireSdk(
		values.commit && values.sha256 && values.evidence && path.isAbsolute(values.evidence),
		'Commit, digest and new evidence required',
	);
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
		filesystem.writeFileSync(path.join(root, 'docs', 'pieces.html'), '<p>café yes</p>');
		filesystem.writeFileSync(
			path.join(root, 'evidence', 'rules', 'paragraph.yml'),
			'id: paragraph\nlanguage: Html\nrule:\n  any:\n    - {kind: document}\n    - {kind: element}\n',
		);
		const check = await observer.run({
			root,
			name: 'check',
			arguments: ['lint', '--config', 'sunstack.lint.json', '--check'],
			expectedExit: 0,
		});
		assert.equal(check.captures.length, 0);
		const listed = await observer.run({
			root,
			name: 'evidence',
			arguments: ['lint', '--config', 'sunstack.lint.json', '--all', '--evidence', 'paragraph'],
			expectedExit: 0,
		});
		assert.equal(listed.captures.length, 0);
		const report = parseObject(listed.stdout.toString('utf8'));
		const pieces = arrayValue(report['pieces']).map(objectValue);
		const spans = pieces.map((piece) => {
			const span = objectValue(objectValue(piece['where'])['span']);
			return [span['byte_start'], span['byte_end']];
		});
		saveSdkEvidence(path.join(values.evidence, 'observed-spans.json'), { spans });
		assert.deepEqual(
			spans,
			[
				[0, 16],
				[0, 16],
			],
			'Candidate did not yield two genuine equal-range pieces',
		);
	} finally {
		if (started) await observer.close();
	}
}
await main();
