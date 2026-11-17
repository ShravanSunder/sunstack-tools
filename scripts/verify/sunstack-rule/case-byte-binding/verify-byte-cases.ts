/** Draft partial real-CLI byte observer. Exit 3 retains the remaining O1–O3 proof gaps. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { objectValue } from '../rulebook/rulebook-fixture-support.ts';
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
import { observeByteCliAcquisition, observeByteSdkAcquisition } from './byte-acquisition.ts';
import { observeByteAdmission } from './byte-admission.ts';
import { observeByteAmbiguity } from './byte-ambiguity.ts';
import { observeArchivedByteNegatives } from './byte-case-negatives.ts';
import { observeByteCliJourney } from './byte-cli-journey.ts';
import { observeInstalledByteCompiler } from './byte-compiler.ts';
import { observeCrlfCase } from './byte-crlf.ts';
import { observeDirectByteCli } from './byte-direct-cli.ts';
import { observeDuplicateByteBinding } from './byte-duplicate.ts';
import { observeRevisionReadback } from './byte-revision-readback.ts';
import { observeInstalledByteSdk } from './byte-sdk.ts';
import { observeStaleDemand } from './byte-stale-demand.ts';
import { observeLegacyCases } from './legacy-cases.ts';
import { observeOldDocumentRefusals } from './old-document-check.ts';

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
		const fixture = await observeByteCliJourney(observer, evidence);
		const root = fixture.root;
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
