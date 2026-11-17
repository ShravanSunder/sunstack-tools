/** Draft partial real-CLI byte observer. Exit 3 retains the remaining O1–O3 proof gaps. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { compactJson, jsonSlice } from '../prepare/raw-json.ts';
import { arrayValue, objectValue, parseObject } from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, responseBody, RulebookProcess } from '../rulebook/rulebook-process.ts';
import {
	confirmSdkTarget,
	containedPath,
	importInstalledSdk,
	inspectInstalledSdkContract,
	requireSdk,
	saveSdkEvidence,
	sdkTargetSchema,
} from '../sdk/sdk-target.ts';
import { createArchiveObserver, requireArchiveRevisions } from './archive-observer.ts';
import { observeByteCliAcquisition, observeByteSdkAcquisition } from './byte-acquisition.ts';
import { observeByteAdmission } from './byte-admission.ts';
import { observeByteAmbiguity } from './byte-ambiguity.ts';
import { observeArchivedByteNegatives } from './byte-case-negatives.ts';
import { observeInstalledByteCompiler } from './byte-compiler.ts';
import { observeCrlfCase } from './byte-crlf.ts';
import { observeDirectByteCli } from './byte-direct-cli.ts';
import { observeDuplicateByteBinding } from './byte-duplicate.ts';
import { createByteFixture } from './byte-fixture.ts';
import { requireByteCaseDetail, requirePreparedByteLinks } from './byte-observation.ts';
import { observeRevisionReadback } from './byte-revision-readback.ts';
import { observeInstalledByteSdk } from './byte-sdk.ts';
import { observeStaleDemand } from './byte-stale-demand.ts';
import { observeLegacyCases } from './legacy-cases.ts';
import { observeOldDocumentRefusals } from './old-document-check.ts';

function preparedPayload(raw: string, id: string): string {
	const report = parseObject(raw);
	const cases = arrayValue(report['cases']).map(objectValue);
	const found = cases.find((item) => item['id'] === id);
	assert.ok(found, `Missing prepared case ${id}`);
	const runs = arrayValue(report['runs']);
	assert.equal(runs.length, 1);
	const requests = arrayValue(objectValue(objectValue(runs[0])['report'])['requests']).map(
		objectValue,
	);
	const index = requests.findIndex((request) => request['id'] === found['result_id']);
	assert.ok(index >= 0, `Missing prepared request for ${id}`);
	return compactJson(jsonSlice(raw, ['runs', 0, 'report', 'requests', index, 'request']));
}

function scoredPayload(raw: string, id: string): string {
	const report = parseObject(raw);
	const found = arrayValue(report['cases'])
		.map(objectValue)
		.find((item) => item['id'] === id);
	assert.ok(found, `Missing scored case ${id}`);
	const runs = arrayValue(report['runs']);
	assert.equal(runs.length, 1);
	const nested = objectValue(objectValue(runs[0])['report']);
	const result = arrayValue(nested['results'])
		.map(objectValue)
		.find((item) => item['id'] === found['result_id']);
	assert.ok(result, `Missing scored result for ${id}`);
	const ids = arrayValue(result['request_ids']);
	assert.equal(ids.length, 1);
	const requests = arrayValue(nested['requests']).map(objectValue);
	const index = requests.findIndex((request) => request['id'] === ids[0]);
	assert.ok(index >= 0, `Missing scored request for ${id}`);
	return compactJson(jsonSlice(raw, ['runs', 0, 'report', 'requests', index, 'request']));
}

async function main(): Promise<void> {
	const { values } = parseArgs({
		strict: true,
		options: {
			target: { type: 'string' },
			evidence: { type: 'string' },
			compiler: { type: 'string' },
			old20: { type: 'string' },
			'old20-sha256': { type: 'string' },
			old21: { type: 'string' },
			'old21-sha256': { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (values.help) {
		process.stdout.write(
			'Draft partial byte observer: --target <installed SDK target JSON including CLI> --compiler <absolute tsc> --old20 <document> --old20-sha256 <hash> --old21 <document> --old21-sha256 <hash> --evidence <new absolute directory>. Requires an independently declared frozen target and explicit run grant. Exit 3 means incomplete O1–O3; see byte-binding reference.\n',
		);
		return;
	}
	requireSdk(values.target && path.isAbsolute(values.target), 'Declare absolute target JSON');
	requireSdk(
		values.evidence && path.isAbsolute(values.evidence),
		'Declare new absolute evidence directory',
	);
	requireSdk(values.compiler && path.isAbsolute(values.compiler), 'Declare absolute compiler');
	requireSdk(
		values.old20 && path.isAbsolute(values.old20) && values.old21 && path.isAbsolute(values.old21),
		'Declare both old document paths',
	);
	requireSdk(
		values['old20-sha256'] &&
			/^[a-f0-9]{64}$/u.test(values['old20-sha256']) &&
			values['old21-sha256'] &&
			/^[a-f0-9]{64}$/u.test(values['old21-sha256']),
		'Declare both old document hashes',
	);
	const evidence = values.evidence;
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
			const input = objectValue(body['state'])['body'];
			const first =
				input === 'café yes' ||
				input === 'end' ||
				input === 'Selected scope.' ||
				input === 'Readable sentinel.';
			const second = input === 'no';
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
		const bothPrepared = filesystem.readFileSync(preparedPath, 'utf8');
		const observeOne = async (id: string, expectedText: string): Promise<void> => {
			const onePreparedPath = path.join(evidence, `prepared-${id}.json`);
			const oneScoredPath = path.join(evidence, `scored-${id}.json`);
			const args = [
				'lint',
				'--config',
				'sunstack.lint.json',
				'cases',
				fixture.casesPath,
				'--set',
				'development',
				'--case',
				id,
			];
			const preparedOne = await observer.run({
				root,
				name: `prepare-${id}`,
				arguments: [...args, '--prepare', onePreparedPath],
				expectedExit: 6,
			});
			assert.equal(preparedOne.captures.length, 0);
			const raw = filesystem.readFileSync(onePreparedPath, 'utf8');
			requirePreparedByteLinks(parseObject(raw), [id]);
			assert.equal(
				preparedPayload(raw, id),
				preparedPayload(bothPrepared, id),
				'Selected prepared payload differs',
			);
			const scoredOne = await observer.run({
				root,
				name: `score-${id}`,
				arguments: [
					'lint',
					'--config',
					'sunstack.lint.json',
					'--report',
					oneScoredPath,
					...args.slice(3),
				],
				credential: true,
				expectedExit: 1,
			});
			assert.equal(scoredOne.captures.length, 1, 'One selected case must dispatch one request');
			const reference = scored.captures.filter(
				(capture) => objectValue(capture.body['state'])['body'] === expectedText,
			);
			assert.equal(reference.length, 1, 'Both-case run did not identify one matching request');
			assert.deepEqual(
				scoredOne.captures[0]?.raw,
				reference[0]?.raw,
				'Selected request payload differs',
			);
			const scoredRaw = filesystem.readFileSync(oneScoredPath, 'utf8');
			assert.equal(
				scoredPayload(scoredRaw, id),
				preparedPayload(raw, id),
				'Prepared and scored request payloads differ',
			);
			const result = arrayValue(parseObject(scoredRaw)['cases']).map(objectValue);
			assert.deepEqual(
				result.map((item) => [item['id'], objectValue(item['outcome'])['kind']]),
				[[id, 'right']],
			);
		};
		await observeOne('first', 'café yes');
		await observeOne('second', 'no');
		const showCase = async (mode: string, file: string, id: string): Promise<void> => {
			const shown = await observer.run({
				root,
				name: `show-${mode}-${id}`,
				arguments: ['lint', 'show', file, id],
				expectedExit: 0,
			});
			const detail = parseObject(shown.stdout.toString('utf8'));
			requireByteCaseDetail(detail, id, fixture.revision);
		};
		await showCase('prepared', preparedPath, 'first');
		await showCase('prepared', preparedPath, 'second');
		await showCase('scored', scoredPath, 'first');
		await showCase('scored', scoredPath, 'second');
		await observeArchivedByteNegatives({ observer, root, evidence, revision: fixture.revision });
		await observeDuplicateByteBinding({ observer, root, evidence, revision: fixture.revision });
		await observeRevisionReadback({
			observer,
			root,
			evidence,
			firstRevision: fixture.revision,
			secondRevision: fixture.unselectedRevision,
		});
		await observeByteAmbiguity({ observer, evidence });
		await observeCrlfCase({ observer, evidence });
		await observeStaleDemand({ observer, evidence });
		await observeByteAdmission({
			observer,
			root,
			evidence,
			firstRevision: fixture.revision,
			secondRevision: fixture.unselectedRevision,
		});
		await observeLegacyCases({ observer, evidence });
		await observeDirectByteCli({ observer, root, evidence: values.evidence });
		await observeByteCliAcquisition({ observer, evidence });
		observeInstalledByteCompiler({ target, compiler: values.compiler, evidence: values.evidence });
		const sdk = await importInstalledSdk(target);
		saveSdkEvidence(
			path.join(values.evidence, 'native-contract.json'),
			await inspectInstalledSdkContract(sdk, target),
		);
		await observeInstalledByteSdk({ sdk, evidence: values.evidence });
		await observeByteSdkAcquisition({ sdk, evidence });
		await observeOldDocumentRefusals({
			sdk,
			evidence,
			documents: [
				{ path: values.old20, sha256: values['old20-sha256'], revision: 20 },
				{ path: values.old21, sha256: values['old21-sha256'], revision: 21 },
			],
		});
		confirmSdkTarget(target);
		saveSdkEvidence(path.join(values.evidence, 'partial.json'), {
			status: 'incomplete',
			observed: [
				'two prepared byte-owner links',
				'two scored case outcomes and prepared/scored byte-specific case readback',
				'installed consumer compilation',
				'installed SDK byte selection, direct geometry and wide codec controls',
				'direct CLI byte selection and geometry controls',
				'archived stale, off-range and partial-codepoint controls',
				'genuine equal-range ambiguity through case preparation and scoring',
				'CRLF whole-document range ending at next line column one',
				'distinct IDs share one exact byte-bound execution',
				'colliding nested result IDs scoped to their own revision',
				'old produced documents refused by the revision-22 native codec',
				'model-backed case selection stale-only and mixed demand',
				'direct SDK model-backed stale demand',
				'file-wide invalid geometry and selected archived bounds refusal',
				'CLI and SDK regular-file successful acquisition counts',
				'legacy line-bound rule and filter case controls',
			],
			gaps: [
				'git2 selection-before-resolution requires a separate source join; archive forwarding observes external Git only',
				'actual child-process absence on selected-bounds refusal',
				'counting-source proof of failed acquisition attempts',
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
