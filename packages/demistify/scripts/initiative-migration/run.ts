import { readFileSync } from 'node:fs';
import { parseArgs } from 'node:util';

import { migrateInitiative } from './migrate-initiative.ts';

const usage = `Usage: node packages/demistify/scripts/initiative-migration/run.ts <initiative-dir> --plan <cut.json> [--dry-run] [--site-root <directory>]
See README.md beside this script.`;

const { values, positionals } = parseArgs({
	allowPositionals: true,
	options: {
		plan: { type: 'string' },
		'dry-run': { type: 'boolean' },
		'site-root': { type: 'string' },
		help: { type: 'boolean' },
	},
});
const [directory] = positionals;
if (values.help) {
	process.stdout.write(
		`${usage}\n\n${readFileSync(new URL('./README.md', import.meta.url), 'utf8')}`,
	);
	process.exit(0);
}
if (!directory || positionals.length !== 1 || !values.plan) {
	process.stderr.write(`${usage}\n`);
	process.exit(1);
}
const migrated = await migrateInitiative(directory, {
	plan: values.plan,
	dryRun: values['dry-run'] ?? false,
	siteRoot: values['site-root'],
});
process.stdout.write(`${migrated.report}\n`);
process.exit(values['dry-run'] || migrated.applied ? 0 : 1);
