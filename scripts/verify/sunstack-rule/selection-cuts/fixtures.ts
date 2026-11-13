/** Reuse the pinned P3/J1/ai-tools corpus; both subjects receive the same headed config. */
import filesystem from 'node:fs';
import path from 'node:path';

import {
	groups,
	headingConfig,
	prepareCopies,
	projectConfigPath,
	requireInputs,
	type CorpusGroup,
} from '../heading-text/fixtures.ts';
export { projectConfigPath, requireInputs };
export function corpusGroups(): CorpusGroup[] {
	return groups().map((group) => ({
		name: group.name,
		config: group.config,
		files: group.files,
		config_sha256: group.config_sha256,
		expected_baseline_requests: group.name === 'html' ? 74 : group.expected_baseline_requests,
	}));
}
export function prepareCorpus(input: string, common: string): void {
	prepareCopies(input, common);
	for (const group of groups().filter(
		(entry) => entry.name === 'html' || entry.name === 'markdown',
	)) {
		const config = path.join(common, 'configs', `${group.name}.json`);
		const bytes = headingConfig(filesystem.readFileSync(config));
		filesystem.writeFileSync(config, bytes);
		filesystem.writeFileSync(projectConfigPath(common, group), bytes);
	}
}
