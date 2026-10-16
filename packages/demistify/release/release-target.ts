import { z } from 'zod';

export interface ReleaseTarget {
	readonly workerName: string;
	readonly hostname: string;
	readonly origin: string;
}
const configurationSchema = z.object({
	name: z.string().regex(/^[a-z0-9][a-z0-9-]{0,62}$/),
	routes: z.tuple([
		z.object({
			pattern: z
				.string()
				.max(253)
				.regex(/^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}$/),
			custom_domain: z.literal(true),
		}),
	]),
});

export function parseReleaseTarget(configuration: unknown): ReleaseTarget {
	const parsed = configurationSchema.parse(configuration);
	const hostname = parsed.routes[0].pattern;
	return { workerName: parsed.name, hostname, origin: `https://${hostname}` };
}

export function verifiedCustomDomainOrigin(target: ReleaseTarget, payload: unknown): string {
	const response = z
		.object({
			success: z.literal(true),
			result: z.array(
				z.object({
					hostname: z.string(),
					service: z.string(),
					environment: z.string(),
				}),
			),
		})
		.safeParse(payload);
	if (!response.success) throw new Error('Cloudflare returned an invalid custom domain response.');
	const matches = response.data.result.filter(
		(binding): boolean => binding.hostname === target.hostname,
	);
	if (
		matches.length !== 1 ||
		matches[0]?.service !== target.workerName ||
		matches[0].environment !== 'production'
	)
		throw new Error(
			'The configured custom domain is not attached to this production Worker; no deploy permitted.',
		);
	return target.origin;
}
