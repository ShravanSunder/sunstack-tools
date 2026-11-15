/** O1–O4 capability qualifier: actual pinned CLI/native SDK; exit3 retains independent joins. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { declaredTarget, releasedTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import {
	confirmSdkTarget,
	containedPath,
	importInstalledSdk,
	inspectInstalledSdkContract,
	saveSdkEvidence,
	sdkTargetSchema,
} from '../sdk/sdk-target.ts';
import { observeMissingCli, showDeletedCli } from './missing-cli.ts';
import { readDeletedSdk, saveMissingSdk } from './missing-codec.ts';
import { compileMissingCredential } from './missing-compiler.ts';
import { MissingCredentialBackend } from './missing-http.ts';
import { observeMissingSdk } from './missing-sdk.ts';

function removeOwnedFixtureFiles(root: string, evidence: string): void {
	assert.ok(containedPath(evidence, root) && root !== evidence);
	assert.equal(filesystem.realpathSync(root), root, 'Owned fixture path moved');
	for (const entry of filesystem.readdirSync(root, { withFileTypes: true })) {
		const file = path.join(root, entry.name);
		assert.ok(!entry.isSymbolicLink(), 'Owned fixture acquired a symlink');
		if (entry.isDirectory()) removeOwnedFixtureFiles(file, evidence);
		else {
			assert.ok(entry.isFile());
			filesystem.unlinkSync(file);
		}
	}
}
async function main(): Promise<void> {
	const { values } = parseArgs({
		strict: true,
		options: {
			target: { type: 'string' },
			compiler: { type: 'string' },
			reference: { type: 'string' },
			evidence: { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (values.help) {
		process.stdout.write(
			'Qualify missing-credential O1–O4: --target <absolute sdkTargetSchema JSON with CLI pin> --reference <absolute JSON: binary,digest,release of preserved released CLI> --compiler <absolute installed tsc> --evidence <new absolute directory>. Requires a quiescent matched source/CLI/packed SDK/addon/compiler bundle, no inherited provider key and an owned external consumer. Uses synthetic fixtures and loopback only. Saves exact transcripts and both native formats; deletes only its owned fixture files before readback. Exit0 is reserved for no open joins; this bounded capability route exits3 with internal-reservation/native-task/CI/release/PR227 joins explicit. Assertion/setup/target drift throws. No Cargo, credentials, paid calls or publication.\n',
		);
		return;
	}
	assert.ok(values.target && path.isAbsolute(values.target));
	assert.ok(values.compiler && path.isAbsolute(values.compiler));
	assert.ok(
		values.reference && path.isAbsolute(values.reference),
		'Declare the preserved released CLI reference manifest',
	);
	assert.ok(values.evidence && path.isAbsolute(values.evidence));
	const evidence = values.evidence;
	const target = sdkTargetSchema.parse(JSON.parse(filesystem.readFileSync(values.target, 'utf8')));
	const referenceInput = z
		.strictObject({
			binary: z.string(),
			digest: z.string().regex(/^[a-f0-9]{64}$/u),
			release: z.string(),
		})
		.parse(JSON.parse(filesystem.readFileSync(values.reference, 'utf8')));
	assert.ok(path.isAbsolute(referenceInput.binary));
	const reference = releasedTarget(
		referenceInput.binary,
		referenceInput.digest,
		referenceInput.release,
	);
	assert.ok(target.cli, 'Matching CLI pin is required');
	assert.ok(
		!containedPath(target.sourceRoot, evidence) && !containedPath(target.consumerRoot, evidence),
	);
	assert.ok(!process.env['OPENROUTER_API_KEY'], 'Refuse inherited provider credential');
	confirmSdkTarget(target);
	filesystem.mkdirSync(evidence, { recursive: false });
	saveSdkEvidence(path.join(evidence, 'target.json'), {
		target,
		reference,
		compiler: values.compiler,
		node: process.version,
		platform: process.platform,
		arch: process.arch,
	});
	compileMissingCredential({ target, compiler: values.compiler, evidence });
	const observer = new RulebookProcess(
		declaredTarget({
			binary: target.cli.binary,
			digest: target.cli.sha256,
			source: target.sourceRoot,
			commit: target.commit,
		}),
		path.join(evidence, 'cli'),
	);
	const backend = new MissingCredentialBackend();
	let cliStarted = false;
	let sdkStarted = false;
	const oldStore = process.env['SUNSTACK_CREDENTIAL_STORE'];
	const oldKey = process.env['OPENROUTER_API_KEY'];
	try {
		delete process.env['OPENROUTER_API_KEY'];
		process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';
		await observer.start();
		cliStarted = true;
		await backend.start();
		sdkStarted = true;
		const sdk = await importInstalledSdk(target);
		saveSdkEvidence(
			path.join(evidence, 'native-contract.json'),
			await inspectInstalledSdkContract(sdk, target),
		);
		const cli = await observeMissingCli(observer, evidence, reference);
		const native = await observeMissingSdk({ sdk, backend, evidence, observer, reference });
		const saved = saveMissingSdk(sdk, native, evidence);
		// Both formats have been saved. Remove all owned fixture bytes, including Git archives.
		removeOwnedFixtureFiles(cli.root, evidence);
		removeOwnedFixtureFiles(native.root, evidence);
		const before = backend.captures.length;
		await readDeletedSdk({ sdk, saved, target, evidence });
		await showDeletedCli(observer, cli);
		assert.equal(backend.captures.length, before, 'Readback dispatched HTTP');
		assert.deepEqual(backend.errors, []);
		confirmSdkTarget(target);
		saveSdkEvidence(path.join(evidence, 'receipt.json'), {
			coverage: 'partial',
			sourceCommit: target.commit,
			contractRevision: target.contractRevision,
			observed: [
				'O1 real CLI/installed SDK independent finding parity and reached unavailable work',
				'O2 nonempty keyed controls, keyless zero HTTP and report accounting, controlled401/400/503, inspection/prepare/invalid/empty/cancellation controls',
				'O3 installed positive/negative narrowing, exact SDK documents and raw CLI reports after fixture deletion, strict codec negatives',
				'O4 actual historical CLI case denominator/result links and installed SDK selected-case identities',
			],
			gaps: [
				'dispatcher private zero-reservation vector needs candidate product-test receipt',
				'candidate facade task-drain/native acquisition/credential-purity tests remain independent joins',
				'required current-head Linux/macOS/native addon/fresh-install CI cells',
				'release and actual PR227 owner consumer receipt',
				'commercial provider quality and live credential store outside controlled fixture claim',
			],
		});
		process.exitCode = 3;
	} catch (error) {
		saveSdkEvidence(path.join(evidence, 'failure.json'), {
			message: error instanceof Error ? error.message : 'Unknown qualification failure',
			stack: error instanceof Error ? error.stack : undefined,
		});
		throw error;
	} finally {
		try {
			if (cliStarted) await observer.close();
		} finally {
			try {
				if (sdkStarted) await backend.close();
			} finally {
				if (oldKey === undefined) delete process.env['OPENROUTER_API_KEY'];
				else process.env['OPENROUTER_API_KEY'] = oldKey;
				if (oldStore === undefined) delete process.env['SUNSTACK_CREDENTIAL_STORE'];
				else process.env['SUNSTACK_CREDENTIAL_STORE'] = oldStore;
				saveSdkEvidence(path.join(evidence, 'native-http.json'), {
					captures: backend.captures,
					errors: backend.errors,
				});
			}
		}
	}
}

await main();
