import { appendFile, readFile } from 'node:fs/promises';

import { parseReleaseTarget, verifiedCustomDomainOrigin } from './release-target.ts';

const token = process.env['CLOUDFLARE_API_TOKEN'];
const accountId = process.env['CLOUDFLARE_ACCOUNT_ID'];
const output = process.env['GITHUB_OUTPUT'];
if (!token || !accountId || !/^[a-f0-9]{32}$/.test(accountId) || !output)
	throw new Error(
		'The protected release job needs its configured Cloudflare secrets and GitHub output file.',
	);
const target = parseReleaseTarget(
	JSON.parse(await readFile('packages/demistify/release/wrangler.json', 'utf8')),
);
// Only Cloudflare's fixed API endpoint receives the token. Confirm the configured
// hostname belongs to this account and production Worker before fetching history.
const response = await fetch(
	`https://api.cloudflare.com/client/v4/accounts/${accountId}/workers/domains?hostname=${encodeURIComponent(target.hostname)}`,
	{
		headers: { Authorization: `Bearer ${token}` },
		redirect: 'error',
		signal: AbortSignal.timeout(20_000),
	},
).catch((): never => {
	throw new Error('Cannot query the configured Cloudflare custom domain binding.');
});
if (!response.ok)
	throw new Error(`Cannot resolve the deployment hostname: HTTP ${response.status}.`);
const payload: unknown = await response.json().catch((): never => {
	throw new Error('Cloudflare returned an unreadable custom domain response.');
});
const origin = verifiedCustomDomainOrigin(target, payload);
await appendFile(output, `origin=${origin}\n`);
process.stdout.write(
	'Bound release history to the configured custom domain, account and production Worker.\n',
);
