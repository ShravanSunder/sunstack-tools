/** Owned asking-rule negatives beside the untouched detached probe input. */
import assert from 'node:assert/strict';
import filesystem from 'node:fs';
import path from 'node:path';

import { prepareRepository } from '../cli-reports/scope-fixtures.ts';
import {
	gitCommand,
	readObject,
	writeJson,
	objectValue,
} from '../rulebook/rulebook-fixture-support.ts';

export function askingFixture(root: string, endpoint: string): string {
	assert.ok(!filesystem.existsSync(root));
	prepareRepository({ root, endpoint });
	const rules = readObject(path.join(root, 'rules.json'));
	const rule = objectValue(rules['K001']);
	rule['questions'] = {
		quality: { type: 'noul', instructions: { check: 'body', task: 'Is this body acceptable?' } },
	};
	rule['decision'] = {
		start: 'quality',
		steps: {
			quality: {
				ask: 'quality',
				yes_at_or_above: 0.8,
				no_at_or_below: 0.6,
				yes: 'pass',
				['no']: { finding: 'Body is not acceptable.' },
				uncertain: 'uncertain',
			},
		},
	};
	writeJson(path.join(root, 'rules.json'), rules);
	gitCommand(root, ['add', '-A']);
	gitCommand(root, ['commit', '-qm', 'Asking rule baseline']);
	return gitCommand(root, ['rev-parse', 'HEAD']);
}
export function requireConfigFree(root: string): void {
	let directory = filesystem.realpathSync(root);
	while (true) {
		assert.ok(
			!filesystem.existsSync(path.join(directory, 'sunstack.lint.json')),
			'No-config probe has an ancestor config',
		);
		const parent = path.dirname(directory);
		if (parent === directory) break;
		directory = parent;
	}
}
