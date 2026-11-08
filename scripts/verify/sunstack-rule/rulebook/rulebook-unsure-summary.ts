/** Compare a declared CLI with an immutable comparator on identical many-unsure inputs. */
import { spawnSync } from 'node:child_process';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireDebugSignature } from './rulebook-amendment.ts';
import {
	arrayValue,
	equal,
	numberValue,
	objectValue,
	readObject,
	repository,
	runSequence,
	requireCondition,
	textValue,
	writeJson,
	type JsonObject,
} from './rulebook-fixture-support.ts';
import {
	requireComparableBytes,
	requireHumanParity,
	requireHumanSummary,
	type SummaryEntry,
} from './rulebook-output-comparison.ts';
import {
	declaredTarget,
	releasedTarget,
	responseBody,
	RulebookProcess,
} from './rulebook-process.ts';

const optionsSchema = z.object({
	binary: z.string(),
	'source-root': z.string().default(repository),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	'baseline-binary': z.string(),
	'baseline-sha256': z.string().regex(/^[a-f0-9]{64}$/),
	'baseline-version': z.string().regex(/^\d+\.\d+\.\d+$/),
	'baseline-kind': z.enum(['released', 'version-only']).default('released'),
	evidence: z.string(),
});

function prepareFixture(root: string): void {
	filesystem.mkdirSync(root);
	filesystem.mkdirSync(path.join(root, 'docs'));
	filesystem.mkdirSync(path.join(root, 'evidence/rules'), { recursive: true });
	filesystem.writeFileSync(path.join(root, 'evidence/sgconfig.yml'), 'ruleDirs: [rules]\n');
	filesystem.writeFileSync(
		path.join(root, 'evidence/rules/pieces.yml'),
		'id: document\nlanguage: Markdown\nrule:\n  kind: document\n',
	);
	filesystem.writeFileSync(
		path.join(root, 'rulebook.md'),
		'Review document. <!-- K001 -->\n\nReview another document. <!-- K002 -->\n',
	);
	const first: JsonObject = {
		name: 'r02',
		evidence: 'review',
		severity: 'warn',
		questions: { quality: { type: 'noul', instructions: { check: 'body' } } },
		decision: {
			start: 'local',
			steps: {
				local: {
					mechanical_check: { tool: 'contains', field: 'body', text: 'FINDING' },
					yes: { finding: 'Keep this warning.' },
					['no']: 'quality',
				},
				quality: {
					ask: 'quality',
					yes_at_or_above: 0.8,
					no_at_or_below: 0.2,
					yes: 'pass',
					['no']: { finding: 'Low quality.' },
					uncertain: 'uncertain',
				},
			},
		},
	};
	const second = structuredClone(first);
	second['name'] = 'other_rule';
	objectValue(objectValue(second['decision'])['steps'])['local'] = {
		mechanical_check: { tool: 'contains', field: 'body', text: 'SECOND_RULE' },
		yes: 'quality',
		['no']: 'pass',
	};
	writeJson(path.join(root, 'rules.json'), { K001: first, K002: second });
	for (let index = 0; index < 66; index++)
		filesystem.writeFileSync(
			path.join(root, `docs/unsure-${String(index).padStart(2, '0')}.md`),
			index < 2 ? 'SECOND_RULE\n' : 'Unsettled evidence.\n',
		);
	for (let index = 0; index < 2; index++)
		filesystem.writeFileSync(path.join(root, `docs/warning-${index}.md`), 'FINDING\n');
}

function requireSignedComparator(binary: string): void {
	if (process.platform !== 'darwin') return;
	const verified = spawnSync('/usr/bin/codesign', ['--verify', '--strict', binary], {
		encoding: 'utf8',
	});
	const signature = spawnSync('/usr/bin/codesign', ['-d', '--verbose=2', binary], {
		encoding: 'utf8',
	});
	requireCondition(
		verified.status === 0 &&
			signature.status === 0 &&
			signature.stderr.includes('Authority=Developer ID Application:'),
		'Comparator lacks a verified Developer ID signature',
	);
}

function summaryEntries(report: JsonObject): SummaryEntry[] {
	return arrayValue(report['results'])
		.map(objectValue)
		.filter((row) => row['outcome'] === 'uncertain')
		.map((row) => ({
			id: numberValue(row['id']),
			file: textValue(row['file']),
			line: numberValue(row['first_line']),
			kind: 'unsure',
			rule: textValue(row['name']),
		}));
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
			'baseline-version': { type: 'string' },
			'baseline-kind': { type: 'string' },
			evidence: { type: 'string' },
		},
	});
	if (values.help) {
		process.stdout.write(
			'Usage: node rulebook-unsure-summary.ts --binary <candidate> --source-root <checkout> --commit <full-head> --sha256 <digest> --baseline-binary <artifact> --baseline-sha256 <digest> --baseline-version <version> --baseline-kind <released|version-only> --evidence <new-directory>\nProves per-rule unsure counts/pointers, unchanged findings/status/exits, all-format/report byte parity and every result ID readback. Equal-version comparison is strict; historical comparison permits only JSON/report root version and SARIF runs[0].tool.driver.version. Model answers are controlled; no real inference. Owns and closes its loopback server and bounded children.\n',
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
	const baseline = releasedTarget(
		options['baseline-binary'],
		options['baseline-sha256'],
		options['baseline-version'],
	);
	requireDebugSignature(target.binary);
	requireSignedComparator(baseline.binary);
	const version = spawnSync(target.binary, ['--version'], { encoding: 'utf8', timeout: 10000 });
	equal(version.status, 0, 'Candidate version probe failed');
	const candidateVersion = z
		.string()
		.regex(/^sunstack \d+\.\d+\.\d+$/)
		.parse(version.stdout.trim())
		.slice('sunstack '.length);
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const root = path.join(evidence, 'fixture');
	prepareFixture(root);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		writeJson(path.join(root, 'skill-rules.config.json'), {
			rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
			marker_prefix: 'K',
			check: { files: ['docs/*.md'] },
			evidence: {
				ast_grep: 'evidence/sgconfig.yml',
				kinds: {
					review: { rule: 'document', max_chars: 1000, fields: { body: { from: 'remainder' } } },
				},
			},
			decision_model: { model: 'jev-latest', endpoint: observer.endpoint },
		});
		observer.reply = (request) => responseBody(request, 0.5);
		const checked = await observer.run({
			root,
			name: 'check',
			arguments: ['lint', '--config', 'skill-rules.config.json', '--check'],
			expectedExit: 0,
		});
		equal(checked.captures.length, 0, 'Keyless check dispatched');
		const reports: JsonObject[] = [];
		await runSequence(['default', 'json', 'github', 'sarif'] as const, async (format) => {
			const beforeFile = path.join(evidence, `baseline-${format}.report.json`);
			const afterFile = path.join(evidence, `candidate-${format}.report.json`);
			const common = [
				'lint',
				'--config',
				'skill-rules.config.json',
				'--all',
				'--format',
				format,
				'--report',
			];
			const before = await observer.run({
				root,
				target: baseline,
				name: `baseline-${format}`,
				arguments: [...common, beforeFile],
				credential: true,
				expectedExit: 5,
			});
			const after = await observer.run({
				root,
				name: `candidate-${format}`,
				arguments: [...common, afterFile],
				credential: true,
				expectedExit: 5,
			});
			equal(before.captures.length, 68, 'Baseline fixture failed to reach every unsure decision');
			equal(after.captures.length, 68, 'Candidate fixture failed to reach every unsure decision');
			const compare = { baselineVersion: options['baseline-version'], candidateVersion };
			requireComparableBytes({
				...compare,
				before: filesystem.readFileSync(beforeFile),
				after: filesystem.readFileSync(afterFile),
				format: 'report',
			});
			const report = readObject(afterFile);
			equal(arrayValue(report['uncertain']).length, 68, 'Unsure results disappeared');
			equal(report['counts'], { error: 0, warn: 2 }, 'Finding counts changed');
			equal(objectValue(report['status'])['kind'], 'uncertain', 'Run status changed');
			if (format === 'default') {
				requireHumanSummary(after.stdout.toString('utf8'), summaryEntries(report));
				requireHumanParity(before.stdout, after.stdout);
				equal(
					before.stdout
						.toString('utf8')
						.split('\n')
						.filter((line) => line.includes(': unsure [')).length,
					68,
					'Comparator did not expose the old individual rows',
				);
			} else
				requireComparableBytes({ ...compare, before: before.stdout, after: after.stdout, format });
			if (format === 'json') reports.push(report);
		});
		const selected = reports[0];
		requireCondition(selected, 'JSON channel was not observed');
		const ids = [
			...arrayValue(selected['results']),
			...arrayValue(selected['filter_answers']),
			...arrayValue(selected['filter_skips']),
		]
			.map(objectValue)
			.map((row) => numberValue(row['id']));
		equal(new Set(ids).size, ids.length, 'Saved result IDs collide');
		equal(ids.length, 136, 'Fixture lost pass/finding/unsure results');
		await runSequence(ids, async (id) => {
			const before = await observer.run({
				root,
				target: baseline,
				name: `baseline-show-${id}`,
				arguments: ['lint', 'show', path.join(evidence, 'baseline-json.report.json'), String(id)],
				expectedExit: 0,
			});
			const after = await observer.run({
				root,
				name: `candidate-show-${id}`,
				arguments: ['lint', 'show', path.join(evidence, 'candidate-json.report.json'), String(id)],
				expectedExit: 0,
			});
			equal(before.captures.length + after.captures.length, 0, 'Saved readback dispatched');
			equal(before.stdout, after.stdout, 'Saved detail bytes changed');
		});
		writeJson(path.join(evidence, 'pass.json'), {
			commit: target.commit,
			binary_sha256: target.digest,
			candidate_version: candidateVersion,
			baseline: { ...baseline, kind: options['baseline-kind'] },
			unsure: 68,
			rules: 2,
			findings: 2,
			all_result_ids: ids,
			readbacks_byte_identical: ids.length,
			http_requests: observer.captures.length,
			version_exceptions:
				candidateVersion === baseline.release
					? []
					: ['JSON/report root version', 'SARIF runs[0].tool.driver.version'],
			not_covered: ['real model quality', 'release acceptance', 'other platforms'],
		});
	} finally {
		await observer.close();
	}
	process.stdout.write('Unsure summary, all channels and every result readback verified.\n');
}
if (process.argv[1] && path.resolve(process.argv[1]) === import.meta.filename) {
	await main();
}
