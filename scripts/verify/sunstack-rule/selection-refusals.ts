/** R40 check-only controls, using canonical S0 mutations with no checked-content acquisition. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	configName,
	configureEndpoint,
	equal,
	requireCondition,
	runSequence,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { type DeclaredTarget, RulebookProcess } from './rulebook-process.ts';
import { copySelectionFixture } from './selection-fixture.ts';

export function prepareSelectionRefusals(options: {
	evidence: string;
	sourceRoot: string;
}): string[] {
	filesystem.mkdirSync(options.evidence);
	const base = copySelectionFixture({
		destination: path.join(options.evidence, 'clean'),
		sourceRoot: options.sourceRoot,
		allKinds: true,
	});
	const sources = filesystem
		.readdirSync(path.join(base, 'refusals'))
		.filter((file) => file.endsWith('.json'))
		.toSorted();
	requireCondition(sources.length >= 18, 'S0 R40 refusal inventory missing');
	for (const file of sources) {
		const root = copySelectionFixture({
			destination: path.join(options.evidence, path.basename(file, '.json')),
			sourceRoot: options.sourceRoot,
			allKinds: true,
		});
		filesystem.copyFileSync(path.join(base, 'refusals', file), path.join(root, configName));
	}
	writeJson(path.join(options.evidence, 'prepared.json'), {
		product_observed: false,
		cases: sources,
		oracle: 'docs/specs/2026-10-30-evidence-selection/specification.md R40',
	});
	return sources.map((file) => path.basename(file, '.json'));
}
export async function observeSelectionRefusals(options: {
	evidence: string;
	sourceRoot: string;
	target: DeclaredTarget;
}): Promise<void> {
	const cases = prepareSelectionRefusals(options);
	const observer = new RulebookProcess(options.target, path.join(options.evidence, 'run'));
	await observer.start();
	const results: JsonObject[] = [];
	try {
		const clean = path.join(options.evidence, 'clean');
		configureEndpoint(clean, observer.endpoint);
		const checked = await observer.run({
			root: clean,
			name: 'selection-clean-check',
			arguments: ['lint', '--config', configName, '--check'],
			expectedExit: 0,
		});
		equal(checked.captures.length, 0, 'Clean check dispatched');
		const text = Buffer.concat([checked.stdout, checked.stderr]).toString('utf8');
		for (const name of ['workflow_section', 'step'])
			requireCondition(text.includes(name), 'Check listing omits kind ' + name);
		await runSequence(cases, async (name) => {
			const root = path.join(options.evidence, name);
			const file = path.join(root, configName);
			filesystem.writeFileSync(
				file,
				filesystem
					.readFileSync(file, 'utf8')
					.replace(
						'"model": "jev-latest"',
						`"model": "jev-latest", "endpoint": "${observer.endpoint}"`,
					),
			);
			const result = await observer.run({
				root,
				name: 'r40-' + name,
				arguments: ['lint', '--config', configName, '--check'],
				expectedExit: 2,
			});
			equal(result.captures.length, 0, 'R40 refusal dispatched');
			const diagnostic = Buffer.concat([result.stdout, result.stderr]).toString('utf8');
			requireCondition(
				diagnostic.includes(path.resolve(file)) && new RegExp(':\\d+(?::\\d+)?:').test(diagnostic),
				'R40 refusal lacks config file/line',
			);
			results.push({ case: name, exit: 2, requests: 0, diagnostic });
		});
	} finally {
		await observer.close();
	}
	writeJson(path.join(options.evidence, 'pass.json'), {
		source_commit: options.target.commit,
		binary_sha256: options.target.digest,
		cases: results,
		required_join:
			'Check signature/call path has no checked-source/capture/backend; native R40 admission assertions distinguish exact causes.',
	});
}
