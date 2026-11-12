// Which input executes, and how uninspectable inputs end, before native dispatch.
import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { Configuration, ConfigurationInput, LoadedConfiguration } from '../src/index.ts';
import { installedBindingSources, resolveBinding } from '../src/native-binding.ts';
import { createSdkCalls } from '../src/sdk-calls.ts';
import { inlineConfiguration, ownedDirectory } from './support/sdk-fixtures.ts';

process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

interface CountedSdk {
	readonly calls: { check: number; run: number };
	readonly sdk: ReturnType<typeof createSdkCalls>;
}

/** The real addon, with native check/run invocations counted. */
function countedSdk(): CountedSdk {
	const calls = { check: 0, run: 0 };
	const sdk = createSdkCalls(() => {
		const resolved = resolveBinding(installedBindingSources());
		if (resolved.kind === 'failed') return resolved;
		const real = resolved.binding;
		return {
			...resolved,
			binding: {
				contractInfo: () => real.contractInfo(),
				CallCancellation: real.CallCancellation,
				load: (options) => real.load(options),
				check: (configuration, cancellation) => {
					calls.check += 1;
					return real.check(configuration, cancellation);
				},
				run: (configuration, request, cancellation) => {
					calls.run += 1;
					return real.run(configuration, request, cancellation);
				},
			},
		};
	});
	return { calls, sdk };
}

/** A value a plain-JavaScript caller can pass where the declarations forbid it. */
function dynamicCallerValue(value: unknown): ConfigurationInput;
function dynamicCallerValue(value: unknown): unknown {
	return value;
}

const base = ownedDirectory('input-boundary');
const configuration: Configuration = inlineConfiguration(
	base.path,
	'http://127.0.0.1:1/v1/systemone',
);

await test('an authored configuration with a configuration key stays authored and is refused by Rust', async () => {
	const { calls, sdk } = countedSdk();
	assert.equal((await sdk.check(configuration)).kind, 'valid');
	// A dynamic caller can add any key; the strict decoder must judge the whole root.
	const smuggled: unknown = { ...configuration, configuration };
	const outcome = await sdk.check(dynamicCallerValue(smuggled));
	assert.equal(outcome.kind, 'invalid');
	if (outcome.kind === 'invalid') {
		assert.equal(outcome.diagnostics[0]?.code, 'unknown_field');
		assert.match(outcome.diagnostics[0]?.message ?? '', /configuration/u);
	}
	assert.equal(calls.check, 2, 'both values reached the real decoder');
});

await test('a loaded value executes its configuration and never its receipt', async () => {
	const { sdk } = countedSdk();
	const loadedShape: LoadedConfiguration = {
		configuration: {
			...configuration,
			rulebooks: [],
			evidence: { kind: 'captured', matchers: {}, utilities: {}, kinds: {} },
		},
		sources: [],
	};
	// rulebooks: [] is invalid for execution, so a valid outcome would mean the receipt or root leaked.
	const outcome = await sdk.check(loadedShape);
	assert.equal(outcome.kind, 'invalid');
});

await test('inputs that throw on inspection end as encoding_rejected with no native call', async () => {
	const { calls, sdk } = countedSdk();
	const throwingWrapper = {
		get configuration(): never {
			throw new Error('synthetic getter refusal');
		},
		sources: [],
	};
	const checked = await sdk.check(dynamicCallerValue(throwingWrapper));
	assert.deepEqual(
		checked.kind === 'failed'
			? [checked.failure.phase, checked.failure.effects, checked.failure.reason]
			: checked.kind,
		['before_dispatch', 'none', 'encoding_rejected'],
	);
	const ran = await sdk.run(dynamicCallerValue(throwingWrapper), {
		root: base.path,
		scope: { kind: 'all' },
	});
	assert.equal(ran.kind === 'failed' ? ran.failure.reason : ran.kind, 'encoding_rejected');

	const throwingOptions = Object.defineProperty(
		{ root: base.path, scope: { kind: 'all' as const } },
		'paths',
		{
			enumerable: true,
			get(): never {
				throw new Error('synthetic paths refusal');
			},
		},
	);
	const options = await sdk.run(configuration, throwingOptions);
	assert.equal(
		options.kind === 'failed' ? options.failure.reason : options.kind,
		'encoding_rejected',
	);
	const throwingSignal = Object.defineProperty({}, 'signal', {
		get(): never {
			throw new Error('synthetic signal refusal');
		},
	});
	const signal = await sdk.check(configuration, throwingSignal);
	assert.equal(signal.kind === 'failed' ? signal.failure.reason : signal.kind, 'encoding_rejected');
	assert.deepEqual(calls, { check: 0, run: 0 });

	// Pre-abort precedence is unchanged.
	assert.deepEqual(
		await sdk.check(dynamicCallerValue(throwingWrapper), { signal: AbortSignal.abort() }),
		{
			kind: 'cancelled',
			diagnostics: [],
		},
	);
});

base.remove();
