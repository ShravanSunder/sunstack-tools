import { appendFile, readFile } from 'node:fs/promises';

import { z } from 'zod';

const token = process.env['CLOUDFLARE_API_TOKEN'];
const accountId = process.env['CLOUDFLARE_ACCOUNT_ID'];
const output = process.env['GITHUB_OUTPUT'];
if (!token || !accountId || !/^[a-f0-9]{32}$/.test(accountId) || !output)
	throw new Error(
		'The protected release job needs its configured Cloudflare secrets and GitHub output file.',
	);
const configuration = z
	.object({ name: z.string().regex(/^[a-z0-9][a-z0-9-]{0,62}$/) })
	.parse(JSON.parse(await readFile('packages/demistify/release/wrangler.json', 'utf8')));
// The same endpoint is used by the pinned Wrangler Workers deployment client.
// No configurable API endpoint or redirects may receive the deployment token.
const response = await fetch(
	`https://api.cloudflare.com/client/v4/accounts/${accountId}/workers/subdomain`,
	{
		headers: { Authorization: `Bearer ${token}` },
		redirect: 'error',
		signal: AbortSignal.timeout(20_000),
	},
).catch((): never => {
	throw new Error('Cannot query the configured Cloudflare Workers hostname.');
});
if (!response.ok)
	throw new Error(`Cannot resolve the deployment hostname: HTTP ${response.status}.`);
const payload: unknown = await response.json().catch((): never => {
	throw new Error('Cloudflare returned an unreadable hostname response.');
});
const result = z
	.object({
		success: z.literal(true),
		result: z.object({
			subdomain: z.string().regex(/^[a-z0-9][a-z0-9-]{0,62}$/),
		}),
	})
	.safeParse(payload);
if (!result.success) throw new Error('Cloudflare did not return a configured Workers hostname.');
const origin = `https://${configuration.name}.${result.data.result.subdomain}.workers.dev`;
await appendFile(output, `origin=${origin}\n`);
process.stdout.write('Bound release history to the configured deployment account and Worker.\n');
