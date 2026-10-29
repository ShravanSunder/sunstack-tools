import assert from 'node:assert/strict';
import path from 'node:path';

import { z } from 'zod';
const channelOptionsSchema = z.object({
	commit: z.string(),
	assets: z.string(),
	'native-root': z.string(),
	fixtures: z.string(),
	evidence: z.string(),
	'dependency-tar': z.string(),
	channel: z.string(),
	pnpm: z.string().optional(),
});
export type ChannelOptions = z.infer<typeof channelOptionsSchema>;

export function argumentsFrom(argv: string[]): ChannelOptions | null {
	if (argv.includes('--help')) {
		process.stdout.write(
			'S4 private installs: --commit DECLARED_PRODUCT_SHA --assets DIR (older/newer plus s4-artifacts.json) --native-root DIR (preflight older/newer/TARGET/sunstack) --fixtures DIR --evidence NEW_DIR --dependency-tar detect-libc-2.1.2.tgz --channel npm|shell|brew [--pnpm PATH]. npm runs local-bin, exec-script, global, npx, exec-npx and pnpm-dlx. No registry credentials or public publication.\n',
		);
		return null;
	}
	const allowed = [
		'commit',
		'assets',
		'native-root',
		'fixtures',
		'evidence',
		'dependency-tar',
		'channel',
		'pnpm',
	];
	const options: Record<string, string> = {};
	for (let index = 0; index < argv.length; index += 2) {
		const key = (argv[index] ?? '').replace(/^--/, '');
		assert.ok(
			allowed.includes(key) && argv[index + 1] && !options[key],
			'unknown/missing/duplicate option',
		);
		options[key] = argv[index + 1] ?? '';
	}
	for (const key of allowed.filter((optionName) => optionName !== 'pnpm'))
		assert.ok(options[key], '--' + key + ' required');
	assert.ok(['npm', 'shell', 'brew'].includes(options['channel'] ?? ''));
	assert.match(options['commit'] ?? '', /^[0-9a-f]{40}$/, 'explicit full declared product commit');
	if (options['channel'] === 'npm')
		assert.ok(options['pnpm'], '--pnpm required for all npm routes');
	for (const key of allowed.filter((optionName) => !['channel', 'commit'].includes(optionName)))
		if (options[key]) options[key] = path.resolve(options[key] ?? '');
	return channelOptionsSchema.parse(options);
}
