/** Run the built story and Map browser cases through the existing Vitest harness. */
import { spawnSync } from 'node:child_process';

import { browserEvidenceRoot } from '../tests/browser/browser-evidence.ts';

const usage = `Usage: node apps/demistify-poc/scripts/verify-story-map.ts [--help]

Run the permanent built story, section, Map and reader-control browser cases.
Run from the monorepo root with Node 26+, the repository's pnpm and installed
matching Chromium. The complete Demistify library and PoC consumer must already
be built from the declared target. This command does not build or install.
DEMISTIFY_BROWSER_ASSETS may select a matching complete library; the consumer
still comes from apps/demistify-poc/dist. The harness copies both into a fresh
snapshot, starts its own loopback server and waits for the document, navigation,
stylesheet, fonts and each case's actual target to be ready.

Keep the command, output, exit status, printed snapshot path, asset identities
and screenshots as evidence. Screenshot files live in a fresh subdirectory of
${browserEvidenceRoot}/. The harness stops its own browser and server. It
checks the two named browser files; supplementary native-host observations,
external navigation, real download saving, physical devices, Safari, cold
recipe application and release acceptance require separate evidence.
`;

const cliArguments = process.argv.slice(2);
if (cliArguments.length === 1 && cliArguments[0] === '--help') {
	process.stdout.write(usage);
} else if (cliArguments.length !== 0) {
	process.stderr.write(usage);
	process.exitCode = 2;
} else {
	const result = spawnSync(
		'pnpm',
		[
			'--filter',
			'@sunstack/demistify-poc',
			'exec',
			'vitest',
			'run',
			'--config',
			'vitest.browser.config.ts',
			'tests/browser/show-me-work.browser.test.ts',
			'tests/browser/show-me-navigation.browser.test.ts',
		],
		{ stdio: 'inherit' },
	);
	if (result.error)
		process.stderr.write(`Unable to start the browser harness: ${result.error.message}\n`);
	if (result.signal) process.stderr.write(`Browser harness ended on signal ${result.signal}.\n`);
	process.exitCode = result.status ?? 1;
}
