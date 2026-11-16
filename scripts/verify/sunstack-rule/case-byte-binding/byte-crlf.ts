/** CRLF document ending at the next line's column one, using archived bytes. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import {
	arrayValue,
	gitCommand,
	objectValue,
	parseObject,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import type { RulebookProcess } from '../rulebook/rulebook-process.ts';
import { byteFixturePath, createByteFixture } from './byte-fixture.ts';

export async function observeCrlfCase(options: {
	readonly observer: RulebookProcess;
	readonly evidence: string;
}): Promise<void> {
	const root = path.join(options.evidence, 'crlf-fixture');
	const fixture = createByteFixture(root, options.observer.endpoint);
	filesystem.writeFileSync(path.join(root, byteFixturePath), '<p>end</p>\r\n');
	filesystem.writeFileSync(
		path.join(root, 'evidence', 'rules', 'paragraph.yml'),
		'id: paragraph\nlanguage: Html\nrule: {kind: document}\n',
	);
	gitCommand(root, ['add', byteFixturePath, 'evidence/rules/paragraph.yml']);
	gitCommand(root, ['commit', '-qm', 'CRLF endpoint']);
	const revision = gitCommand(root, ['rev-parse', 'HEAD']);
	writeJson(fixture.casesPath, {
		repository: root,
		cases: [
			{
				id: 'crlf',
				kind: 'rule',
				rule: 'approval',
				file: byteFixturePath,
				revision,
				lines: [1, 1],
				bytes: [0, 12],
				text_sha256: 'ece2976e6aedd8f8d61eb19a81e06f05686e75183fa139f078042aa33dc85866',
				expected: 'pass',
				set: 'development',
				label_from: 'blind-agents',
			},
		],
		bars: [],
	});
	const evidence = await options.observer.run({
		root,
		name: 'crlf-evidence',
		arguments: ['lint', '--config', 'sunstack.lint.json', '--all', '--evidence', 'paragraph'],
		expectedExit: 0,
	});
	const pieces = arrayValue(parseObject(evidence.stdout.toString('utf8'))['pieces']).map(
		objectValue,
	);
	assert.equal(pieces.length, 1);
	const location = objectValue(pieces[0]?.['where']);
	const span = objectValue(location['span']);
	assert.deepEqual([location['first_line'], location['last_line']], [1, 1]);
	assert.deepEqual(
		[span['byte_start'], span['byte_end'], span['end_line'], span['end_column']],
		[0, 12, 2, 1],
	);
	const observe = async (mode: 'prepare' | 'score'): Promise<void> => {
		const reportPath = path.join(options.evidence, `crlf-${mode}.json`);
		const args =
			mode === 'prepare'
				? [
						'lint',
						'--config',
						'sunstack.lint.json',
						'cases',
						fixture.casesPath,
						'--prepare',
						reportPath,
					]
				: [
						'lint',
						'--config',
						'sunstack.lint.json',
						'--report',
						reportPath,
						'cases',
						fixture.casesPath,
					];
		const result = await options.observer.run({
			root,
			name: `crlf-${mode}`,
			arguments: args,
			credential: mode === 'score',
			expectedExit: mode === 'prepare' ? 6 : 1,
		});
		assert.equal(result.captures.length, mode === 'prepare' ? 0 : 1);
		const rows = arrayValue(parseObject(filesystem.readFileSync(reportPath, 'utf8'))['cases']).map(
			objectValue,
		);
		assert.equal(rows.length, 1);
		assert.equal(
			objectValue(rows[0]?.['outcome'])['kind'],
			mode === 'prepare' ? 'prepared' : 'right',
		);
	};
	await observe('prepare');
	await observe('score');
}
