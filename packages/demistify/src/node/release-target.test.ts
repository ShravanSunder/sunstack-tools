import { describe, expect, it } from 'vitest';

import { parseReleaseTarget, verifiedCustomDomainOrigin } from '../../release/release-target.ts';

const target = {
	name: 'sunstack-demistify',
	routes: [{ pattern: 'demistify.getagentstudio.dev', custom_domain: true }],
};
const binding = {
	hostname: 'demistify.getagentstudio.dev',
	service: 'sunstack-demistify',
	environment: 'production',
};

describe('configured CDN history ownership', (): void => {
	it('uses the configured custom origin only after its production Worker binding matches', (): void => {
		expect(
			verifiedCustomDomainOrigin(parseReleaseTarget(target), {
				success: true,
				result: [binding],
			}),
		).toBe('https://demistify.getagentstudio.dev');
	});
	const rejectedBindings = [
		{ reason: 'missing binding', bindings: [] },
		{ reason: 'another Worker', bindings: [{ ...binding, service: 'another-worker' }] },
		{ reason: 'staging environment', bindings: [{ ...binding, environment: 'staging' }] },
		{ reason: 'ambiguous bindings', bindings: [binding, binding] },
	];
	it.each(rejectedBindings)(
		'refuses $reason',
		({ bindings }: (typeof rejectedBindings)[number]): void => {
			expect((): string =>
				verifiedCustomDomainOrigin(parseReleaseTarget(target), {
					success: true,
					result: bindings,
				}),
			).toThrow('not attached to this production Worker');
		},
	);
	it('rejects path routes and multiple targets before looking up history', (): void => {
		expect(() =>
			parseReleaseTarget({
				...target,
				routes: [{ pattern: 'demistify.getagentstudio.dev/*', custom_domain: true }],
			}),
		).toThrow();
		expect(() =>
			parseReleaseTarget({ ...target, routes: [...target.routes, ...target.routes] }),
		).toThrow();
	});
});
