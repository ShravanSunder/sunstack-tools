/** Draft partial real-CLI byte observer. Exit 3 retains the remaining O1–O3 proof gaps. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { arrayValue, objectValue, parseObject } from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, responseBody, RulebookProcess } from '../rulebook/rulebook-process.ts';
import {
	confirmSdkTarget,
	containedPath,
	requireSdk,
	saveSdkEvidence,
	sdkTargetSchema,
} from '../sdk/sdk-target.ts';
import { createArchiveObserver, requireArchiveRevisions } from './archive-observer.ts';
import { observeInstalledByteCompiler } from './byte-compiler.ts';
import { createByteFixture } from './byte-fixture.ts';
import { requirePreparedByteLinks } from './byte-observation.ts';

async function main(): Promise<void> {
	const { values } = parseArgs({
		strict: true,
		options: {
			target: { type: 'string' },
			evidence: { type: 'string' },
			compiler: { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (values.help) {
		process.stdout.write(
			'Draft partial byte observer: --target <installed SDK target JSON including CLI> --compiler <absolute tsc> --evidence <new absolute directory>. Requires an independently declared frozen target and explicit run grant. Exit 3 means incomplete O1–O3; see byte-binding reference.\n',
		);
		return;
	}
	requireSdk(values.target && path.isAbsolute(values.target), 'Declare absolute target JSON');
	requireSdk(
		values.evidence && path.isAbsolute(values.evidence),
		'Declare new absolute evidence directory',
	);
	requireSdk(values.compiler && path.isAbsolute(values.compiler), 'Declare absolute compiler');
	const target = sdkTargetSchema.parse(JSON.parse(filesystem.readFileSync(values.target, 'utf8')));
	requireSdk(target.cli, 'Target must include CLI');
	requireSdk(
		!containedPath(target.sourceRoot, values.evidence) &&
			!containedPath(target.consumerRoot, values.evidence),
		'Evidence must be external to source and consumer',
	);
	confirmSdkTarget(target);
	const cli = declaredTarget({
		binary: target.cli.binary,
		digest: target.cli.sha256,
		source: target.sourceRoot,
		commit: target.commit,
	});
	filesystem.mkdirSync(values.evidence, { recursive: false });
	saveSdkEvidence(path.join(values.evidence, 'target.json'), target);
	const observer = new RulebookProcess(cli, path.join(values.evidence, 'cli'));
	let started = false;
	try {
		await observer.start();
		started = true;
		observer.respond = async ({ body }) => {
			const input = JSON.stringify(body);
			const first = input.includes('café yes');
			const second = input.includes('>no<');
			assert.notEqual(first, second, 'Request did not identify exactly one fixture paragraph');
			return { status: 200, body: responseBody(body, first ? 0.91 : 0.09) };
		};
		const root = path.join(values.evidence, 'fixture');
		const fixture = createByteFixture(root, observer.endpoint);
		const archive = createArchiveObserver(
			path.join(values.evidence, 'git-forwarder'),
			'/usr/bin/git',
		);
		const preparedPath = path.join(values.evidence, 'prepared.json');
		const scoredPath = path.join(values.evidence, 'scored.json');
		const selected = [
			'lint',
			'--config',
			'sunstack.lint.json',
			'cases',
			fixture.casesPath,
			'--set',
			'development',
			'--case',
			'first',
			'--case',
			'second',
		];
		const prepared = await observer.run({
			root,
			name: 'both-prepare',
			executableDirectory: archive.executableDirectory,
			arguments: [...selected, '--prepare', preparedPath],
			expectedExit: 6,
		});
		assert.equal(prepared.captures.length, 0, 'Prepare dispatched a request');
		requireArchiveRevisions(archive.archiveLog, [fixture.revision]);
		assert.notEqual(fixture.revision, fixture.unselectedRevision);
		requirePreparedByteLinks(parseObject(filesystem.readFileSync(preparedPath, 'utf8')), [
			'first',
			'second',
		]);
		const scored = await observer.run({
			root,
			name: 'both-score',
			arguments: [
				'lint',
				'--config',
				'sunstack.lint.json',
				'--report',
				scoredPath,
				...selected.slice(3),
			],
			credential: true,
			expectedExit: 1,
		});
		assert.equal(scored.captures.length, 2, 'Scoring did not dispatch two distinct requests');
		const report = parseObject(filesystem.readFileSync(scoredPath, 'utf8'));
		const cases = arrayValue(report['cases']).map(objectValue);
		assert.deepEqual(
			cases.map((item) => [item['id'], objectValue(item['outcome'])['kind']]),
			[
				['first', 'right'],
				['second', 'right'],
			],
		);
		const showCase = async (id: string): Promise<void> => {
			const shown = await observer.run({
				root,
				name: `show-${id}`,
				arguments: ['lint', 'show', scoredPath, id],
				expectedExit: 0,
			});
			const detail = parseObject(shown.stdout.toString('utf8'));
			assert.equal(objectValue(detail['case'])['id'], id);
		};
		await showCase('first');
		await showCase('second');
		observeInstalledByteCompiler({ target, compiler: values.compiler, evidence: values.evidence });
		confirmSdkTarget(target);
		saveSdkEvidence(path.join(values.evidence, 'partial.json'), {
			status: 'incomplete',
			observed: [
				'two prepared byte-owner links',
				'two scored case outcomes and case-selector readback',
				'installed consumer compilation',
			],
			gaps: [
				'individual selection and payload parity',
				'scored byte-specific result join',
				'two-revision colliding-ID readback',
				'O2 matrix and archive observation',
				'direct SDK and CLI runtime geometry and stale controls',
				'SDK bigint transport and genuine old documents',
			],
		});
		process.exitCode = 3;
	} catch (error) {
		saveSdkEvidence(path.join(values.evidence, 'failure.json'), {
			message: error instanceof Error ? error.message : String(error),
		});
		throw error;
	} finally {
		if (started) await observer.close();
	}
}

await main();
