/** Bounded draft observer. A successful partial run deliberately exits incomplete. */
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { declaredTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { SdkControlledBackend } from '../sdk/sdk-controlled-backend.ts';
import {
	confirmSdkTarget,
	containedPath,
	importInstalledSdk,
	inspectInstalledSdkContract,
	requireSdk,
	saveSdkEvidence,
	sdkTargetSchema,
} from '../sdk/sdk-target.ts';
import { observeInstalledSelectionCompiler } from './case-dispatch-compiler.ts';
import { observeCorpusSelectedPreparation } from './case-dispatch-corpus-prepare.ts';
import { observeFilterDispatch } from './case-dispatch-filters.ts';
import { observeSameFileCaseDispatch } from './case-dispatch-journey.ts';
import { observePendingCaseSelection } from './case-dispatch-pending.ts';
import { observeInstalledCaseDispatch } from './case-dispatch-sdk.ts';

async function main(): Promise<void> {
	const { values } = parseArgs({
		strict: true,
		options: {
			target: { type: 'string' },
			evidence: { type: 'string' },
			compiler: { type: 'string' },
			manifest: { type: 'string' },
			copies: { type: 'string' },
			'rulebook-config': { type: 'string' },
			'instrument-pins': { type: 'string' },
			'prepared-exit': { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (values.help) {
		process.stdout.write(
			'Draft partial observer: --target <installed SDK target JSON with CLI pin> --compiler <absolute tsc> --evidence <new absolute directory>. Optional corpus preparation requires --manifest <original dispatch.json> --copies <directory> --rulebook-config <absolute config> --instrument-pins <absolute preflight.json> --prepared-exit <code>. Exit 3 means remaining proof gaps. Run only on a frozen, separately commissioned target.\n',
		);
		return;
	}
	requireSdk(
		values.target && path.isAbsolute(values.target),
		'Declare an absolute installed target JSON',
	);
	requireSdk(
		values.evidence && path.isAbsolute(values.evidence),
		'Declare an absolute new evidence directory',
	);
	requireSdk(
		values.compiler && path.isAbsolute(values.compiler),
		'Declare the installed TypeScript compiler',
	);
	const target = sdkTargetSchema.parse(JSON.parse(filesystem.readFileSync(values.target, 'utf8')));
	requireSdk(target.cli, 'Target must pin its matching CLI');
	requireSdk(
		!containedPath(target.sourceRoot, values.evidence) &&
			!containedPath(target.consumerRoot, values.evidence),
		'Evidence must be external to product and consumer',
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
	let observerStarted = false;
	const backend = new SdkControlledBackend();
	let backendStarted = false;
	const oldKey = process.env['OPENROUTER_API_KEY'];
	const oldStore = process.env['SUNSTACK_CREDENTIAL_STORE'];
	try {
		requireSdk(!oldKey, 'Controlled observer requires no inherited provider key');
		process.env['OPENROUTER_API_KEY'] = 'SYNTHETIC_SDK_LOOPBACK_ONLY';
		process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';
		await observer.start();
		observerStarted = true;
		await observeSameFileCaseDispatch({ observer, evidence: observer.evidence });
		await observeFilterDispatch({ observer, evidence: observer.evidence });
		await observePendingCaseSelection({
			observer,
			evidence: observer.evidence,
			sourceRoot: target.sourceRoot,
		});
		const corpusOptions = [
			values.manifest,
			values.copies,
			values['rulebook-config'],
			values['instrument-pins'],
			values['prepared-exit'],
		];
		if (corpusOptions.some((value) => value !== undefined)) {
			requireSdk(
				values.manifest &&
					values.copies &&
					values['rulebook-config'] &&
					values['instrument-pins'] &&
					values['prepared-exit'],
				'Declare all corpus observation inputs together',
			);
			const preparedExit = Number(values['prepared-exit']);
			requireSdk(
				Number.isInteger(preparedExit) && preparedExit >= 0 && preparedExit <= 255,
				'Invalid declared prepared exit',
			);
			await observeCorpusSelectedPreparation({
				observer,
				evidence: observer.evidence,
				manifest: values.manifest,
				copies: values.copies,
				rulebookConfig: values['rulebook-config'],
				instrumentPins: values['instrument-pins'],
				preparedExit,
			});
		}
		await backend.start();
		backendStarted = true;
		const sdk = await importInstalledSdk(target);
		saveSdkEvidence(
			path.join(values.evidence, 'native-contract.json'),
			await inspectInstalledSdkContract(sdk, target),
		);
		await observeInstalledCaseDispatch(
			{ sdk, target, evidence: values.evidence, backend },
			observer,
		);
		observeInstalledSelectionCompiler({
			target,
			evidence: values.evidence,
			compiler: values.compiler,
		});
		requireSdk(backend.errors.length === 0, 'Controlled SDK backend reported errors');
		confirmSdkTarget(target);
		saveSdkEvidence(path.join(values.evidence, 'partial.json'), {
			status: 'incomplete',
			observed: [
				'same-file selected/excluded CLI and installed SDK control',
				'standalone selected-filter prepare and controlled HTTP inventory',
				'gated-rule dependency and separately selected other-folder HTTP witness',
				'stale filter and selected-case cancellation accounting',
				'pending selection prerequisite and case disposition',
				'positive and negative selection settlement with ordinary request parity',
				'incomplete selected-case accounting',
				'installed positive and negative selection caller compiles',
			],
			gaps: ['real corpus prepare parity'],
		});
		process.exitCode = 3;
	} catch (error) {
		saveSdkEvidence(path.join(values.evidence, 'failure.json'), {
			message: error instanceof Error ? error.message : 'Unknown observation failure',
		});
		throw error;
	} finally {
		if (observerStarted) await observer.close();
		if (backendStarted) await backend.close();
		if (oldKey === undefined) delete process.env['OPENROUTER_API_KEY'];
		else process.env['OPENROUTER_API_KEY'] = oldKey;
		if (oldStore === undefined) delete process.env['SUNSTACK_CREDENTIAL_STORE'];
		else process.env['SUNSTACK_CREDENTIAL_STORE'] = oldStore;
		saveSdkEvidence(path.join(values.evidence, 'sdk-http.json'), {
			captures: backend.captures,
			errors: backend.errors,
		});
	}
}

await main();
