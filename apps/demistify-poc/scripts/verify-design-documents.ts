/** Run the public document-browser cases through the existing built-fixture harness. */
import { spawnSync } from 'node:child_process';

import { browserEvidenceRoot } from '../tests/browser/browser-evidence.ts';

const usage = `Usage: node apps/demistify-poc/scripts/verify-design-documents.ts [--help]

Run the permanent document-browser, collection-entry, drawer and Comments
browser files through the existing Vitest built-fixture harness. This route is
operable for those files’ assertions. Read the source of each selected file and
report its actual assertions; file selection alone does not prove every
document-browser obligation.

Run from the monorepo root with Node 26+, pnpm and the installed matching
Chromium. The complete Demistify library and PoC consumer must already be
built from the declared target. This command does not build or install.
DEMISTIFY_BROWSER_ASSETS may select a matching complete library; the consumer
still comes from apps/demistify-poc/dist. The harness snapshots the assets and
starts its own loopback server. It waits for its real document/navigation,
stylesheet, fonts and each case's target rather than treating a port as ready.

Keep the command, output, selected pass/fail/skip counts, exit, snapshot path,
asset identities and screenshots/metrics. Browser evidence uses a fresh
subdirectory of ${browserEvidenceRoot}/. The harness owns its server and
browser cleanup. Actual release, independent first-author work, all CLI cases,
full source fidelity, scroller reachability, physical devices, Safari and any
native behavior not exercised by these files need separate evidence. Confirm
cleanup of owned processes; a closed port alone does not establish every child
has exited. A grey or unavailable screenshot is not a visual pass.
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
			'tests/browser/document-browsers.browser.test.ts',
			'tests/browser/collection-entry.browser.test.ts',
			'tests/browser/drawer.browser.test.ts',
			'tests/browser/choices-comments.browser.test.ts',
		],
		{ stdio: 'inherit' },
	);
	if (result.error)
		process.stderr.write(`Unable to start the browser harness: ${result.error.message}\n`);
	if (result.signal) process.stderr.write(`Browser harness ended on signal ${result.signal}.\n`);
	process.exitCode = result.status ?? 1;
}
