/** U9 through a frozen binary. --prepare does not drive a product; no real credentials. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	configName,
	configureEndpoint,
	equal,
	errorMessage,
	journeyArguments,
	runSequence,
	oracle,
	prepareFixture,
	requireCondition,
	rulebookName,
	rulesName,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from './rulebook-process.ts';
import { prepareCases, prepareFifo } from './rulebook-refusal-fixtures.ts';
import { observeSelectionRefusals, prepareSelectionRefusals } from './selection-refusals.ts';

function locatedInputs(root: string): Set<string> {
	const files = [
		configName,
		rulebookName,
		rulesName,
		'evidence/sgconfig.yml',
		'absent.yml',
		'absent.json',
	].map((file) => path.resolve(root, file));
	for (const directory of ['evidence/rules', 'evidence/utils'])
		for (const file of filesystem.readdirSync(path.join(root, directory)))
			if (file.endsWith('.yml')) files.push(path.resolve(root, directory, file));
	return new Set(files);
}
export async function main(): Promise<void> {
	const options = journeyArguments(
		'Rulebook --check: R24 refusals plus bounded no-writer FIFO, no key.',
	);
	if (!options) return;
	if (options.case.includes('selection')) {
		if (options.prepare) {
			prepareSelectionRefusals({
				evidence: path.resolve(options.evidence),
				sourceRoot: options['source-root'],
			});
		} else {
			const target = declaredTarget({
				binary: options.binary ?? '',
				source: options['source-root'],
				commit: options.commit ?? '',
				...(options.sha256 ? { digest: options.sha256 } : {}),
			});
			await observeSelectionRefusals({
				evidence: path.resolve(options.evidence),
				sourceRoot: options['source-root'],
				target,
			});
		}
		return;
	}
	const target = options.prepare
		? undefined
		: declaredTarget({
				binary: options.binary ?? '',
				source: options['source-root'],
				commit: options.commit ?? '',
				...(options.sha256 ? { digest: options.sha256 } : {}),
			});
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence, { recursive: false });
	const cases = prepareCases(path.join(evidence, 'fixtures'));
	const fifo = prepareFifo(path.join(evidence, 'fifo-check'));
	if (options.prepare) {
		writeJson(path.join(evidence, 'prepared.json'), {
			product_observed: false,
			cases: cases.length,
			oracle,
			fifo_without_writer_outside_git: true,
		});
		process.stdout.write(`Prepared ${cases.length} R24 controls; product unobserved\n`);
		return;
	}
	requireCondition(target, 'Declared target missing');
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	const results: JsonObject[] = [];
	try {
		const clean = prepareFixture(path.join(evidence, 'clean'));
		configureEndpoint(clean, observer.endpoint);
		await runSequence(
			[
				['clean-check', clean],
				['fifo-no-writer-check', fifo],
			],
			async ([name, root]) => {
				requireCondition(name && root, 'Missing check fixture');
				configureEndpoint(root, observer.endpoint);
				const result = await observer.run({
					root,
					name,
					arguments: ['lint', '--config', configName, '--check'],
					expectedExit: 0,
				});
				const text = Buffer.concat([result.stdout, result.stderr]).toString('utf8');
				requireCondition(
					result.captures.length === 0 &&
						['K001', 'K002', 'K003'].every((marker) => text.includes(marker)),
					'Clean/FIFO check lost rule listing or dispatched',
				);
			},
		);
		await runSequence(cases, async (entry) => {
			const root = path.join(evidence, 'fixtures', entry.name);
			const config = path.join(root, configName);
			// Text replacement preserves malformed and duplicate JSON evidence.
			filesystem.writeFileSync(
				config,
				filesystem
					.readFileSync(config, 'utf8')
					.replaceAll('http://127.0.0.1:9/systemone', observer.endpoint),
			);
			const result = await observer.run({
				root,
				name: entry.name,
				arguments: ['lint', '--config', configName, '--check'],
				expectedExit: 2,
			});
			const text = Buffer.concat([result.stdout, result.stderr]).toString('utf8');
			requireCondition(result.captures.length === 0, `${entry.name}: check sent HTTP`);
			const locations = [...text.matchAll(/^(.+?):(\d+)(?::(\d+))?:/gm)];
			const admitted = locatedInputs(root);
			requireCondition(
				locations.length > 0 &&
					locations.every(
						(match) => admitted.has(path.resolve(match[1] ?? '')) && Number(match[2]) >= 1,
					),
				`${entry.name}: diagnostic not located in a related input`,
			);
			if (entry.name === 'config-duplicate') {
				const line =
					filesystem
						.readFileSync(config, 'utf8')
						.split('\n')
						.findIndex((value) => value.split('"marker_prefix"').length === 3) + 1;
				requireCondition(
					text.toLowerCase().includes('duplicate key') && text.includes(`:${line}:`),
					'Duplicate key not located at its declaration',
				);
			}
			if (entry.name === 'evidence-max-under-floor') {
				const lines = filesystem.readFileSync(config, 'utf8').split('\n');
				const keyLines = lines.flatMap((line, index) =>
					line.startsWith('    "evidence_max_chars":') ? [index + 1] : [],
				);
				equal(keyLines.length, 1, 'Low evidence limit fixture must have one authored limit key');
				requireCondition(
					text.includes(`${path.resolve(config)}:${keyLines[0]}:5:`),
					'Low evidence limit refusal must locate its actual key under limits',
				);
			}
			if (entry.name.startsWith('old-'))
				requireCondition(
					text.includes(entry.offending) && text.includes(entry.reason),
					`${entry.name}: old key/replacement not named`,
				);
			results.push({
				case: entry.name,
				exit: 2,
				http_requests: 0,
				identifier_named: text.toLowerCase().includes(entry.offending.toLowerCase()),
				diagnostic: text.trim(),
			});
		});
	} finally {
		await observer.close();
	}
	writeJson(path.join(evidence, 'pass.json'), {
		commit: target.commit,
		binary_sha256: target.digest,
		cases: results,
		oracle,
		required_join:
			'U9 structural check call path has no checked source/capture/backend; FIFO and located refusals observed. Run acquisition gates remain separate.',
	});
	process.stdout.write(
		`Observed ${results.length} refusal cases and bounded FIFO check; structural join remains separate\n`,
	);
}
if (import.meta.main)
	try {
		await main();
	} catch (error) {
		process.stderr.write(`Rulebook check observation failed: ${errorMessage(error)}\n`);
		process.exitCode = 2;
	}
