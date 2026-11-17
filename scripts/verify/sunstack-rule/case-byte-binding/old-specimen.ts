/** Draft: create a synthetic document with an actual separately installed old SDK writer. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { sdkInlineConfiguration } from '../sdk/sdk-fixtures.ts';

async function main(): Promise<void> {
	const { values } = parseArgs({
		strict: true,
		options: {
			entry: { type: 'string' },
			output: { type: 'string' },
			root: { type: 'string' },
			revision: { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (values.help) {
		process.stdout.write(
			'Create one synthetic SDK document using an independently pinned old installed SDK: --entry <absolute installed entry> --root <absolute synthetic root> --revision <20|21> --output <new absolute file>. Requires separate old-codec execution grant; retain artifact hashes and installed consumer identity.\n',
		);
		return;
	}
	assert.ok(values.entry && path.isAbsolute(values.entry));
	assert.ok(values.output && path.isAbsolute(values.output));
	assert.ok(values.root && path.isAbsolute(values.root));
	assert.ok(values.revision === '20' || values.revision === '21');
	const syncCall = z.custom<(...args: unknown[]) => unknown>(
		(value) => typeof value === 'function',
	);
	const asyncCall = z.custom<(...args: unknown[]) => Promise<unknown>>(
		(value) => typeof value === 'function',
	);
	const oldSdk = z
		.object({ serialize: syncCall, deserialize: asyncCall, contractInfo: asyncCall })
		.parse(await import(pathToFileURL(values.entry).href));
	const identity = z
		.object({
			kind: z.literal('ready'),
			contract: z.object({
				contract_revision: z.number(),
				sdk_release: z.string(),
				target: z.string(),
			}),
		})
		.parse(await oldSdk.contractInfo());
	assert.equal(identity.contract.contract_revision, Number(values.revision));
	const input = sdkInlineConfiguration(values.root, 'http://127.0.0.1:9/systemone', 'SYNTHETIC_E5');
	const output = z
		.object({ kind: z.literal('serialized'), json: z.string() })
		.parse(oldSdk.serialize({ kind: 'configuration', value: input }));
	const document = z
		.object({ contract_revision: z.number().int(), payload: z.unknown() })
		.parse(JSON.parse(output.json));
	assert.equal(
		document.contract_revision,
		Number(values.revision),
		'Old writer returned another revision',
	);
	const decoded = z
		.object({ kind: z.literal('decoded') })
		.parse(await oldSdk.deserialize(output.json));
	filesystem.writeFileSync(values.output, output.json, { flag: 'wx', mode: 0o600 });
	process.stdout.write(
		`${JSON.stringify({ contract: identity.contract, readback: decoded.kind })}\n`,
	);
}
await main();
