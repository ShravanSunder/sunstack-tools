/** Legacy line and filter cases keep their original selectors and digests. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { casesConfigName, prepareCasesFixture } from '../rulebook/rulebook-case-fixtures.ts';
import {
	arrayValue,
	objectValue,
	parseObject,
	readObject,
	textValue,
	writeJson,
} from '../rulebook/rulebook-fixture-support.ts';
import { responseBody, type RulebookProcess } from '../rulebook/rulebook-process.ts';

export async function observeLegacyCases(options: {
	readonly observer: RulebookProcess;
	readonly evidence: string;
}): Promise<void> {
	const root = path.join(options.evidence, 'legacy-fixture');
	const fixture = prepareCasesFixture(root);
	const lineCase = arrayValue(fixture['rule_cases'])
		.map(objectValue)
		.find((row) => row['id'] === 'pass');
	const filterCase = arrayValue(fixture['filter_cases'])
		.map(objectValue)
		.find((row) => row['id'] === 'filter-right');
	assert.ok(lineCase && filterCase);
	const configPath = path.join(root, casesConfigName);
	const config = readObject(configPath);
	objectValue(config['decision_model'])['endpoint'] = options.observer.endpoint;
	writeJson(configPath, config);
	const caseFile = path.join(options.evidence, 'legacy-cases.json');
	writeJson(caseFile, { repository: root, cases: [lineCase, filterCase], bars: [] });
	const previous = options.observer.respond;
	options.observer.respond = async ({ body }) => ({
		status: 200,
		body: responseBody(body, 0.95, 'reusable'),
	});
	try {
		const observe = async (id: string): Promise<void> => {
			const preparedPath = path.join(options.evidence, `legacy-${id}-prepared.json`);
			const prepared = await options.observer.run({
				root,
				name: `legacy-${id}-prepare`,
				arguments: [
					'lint',
					'--config',
					casesConfigName,
					'cases',
					caseFile,
					'--case',
					id,
					'--prepare',
					preparedPath,
				],
				expectedExit: 6,
			});
			assert.equal(prepared.captures.length, 0);
			const prepRows = arrayValue(
				parseObject(filesystem.readFileSync(preparedPath, 'utf8'))['cases'],
			).map(objectValue);
			assert.equal(prepRows.length, 1);
			assert.equal(objectValue(prepRows[0]?.['outcome'])['kind'], 'prepared');
			const scoredPath = path.join(options.evidence, `legacy-${id}-scored.json`);
			const scored = await options.observer.run({
				root,
				name: `legacy-${id}-score`,
				arguments: [
					'lint',
					'--config',
					casesConfigName,
					'--report',
					scoredPath,
					'cases',
					caseFile,
					'--case',
					id,
				],
				credential: true,
				expectedExit: 1,
			});
			assert.ok(scored.captures.length > 0, 'Legacy positive did not dispatch');
			const rows = arrayValue(
				parseObject(filesystem.readFileSync(scoredPath, 'utf8'))['cases'],
			).map(objectValue);
			assert.equal(rows.length, 1);
			assert.equal(textValue(rows[0]?.['id']), id);
			assert.equal(objectValue(rows[0]?.['outcome'])['kind'], 'right');
		};
		await observe('pass');
		await observe('filter-right');
	} finally {
		options.observer.respond = previous;
	}
}
