/** Check every pinned real Markdown fragment link and section field through a declared signed CLI. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireOutsideInput } from '../git-first/frozen-inputs.ts';
import { writeProject } from '../html-evidence/html-fixtures.ts';
import { requireDebugSignature } from '../rulebook/rulebook-amendment.ts';
import {
	objectValue,
	readObject,
	writeJson,
	runSequence,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { requireLinks } from './checks.ts';
import { repositories, requireFile, requireInputs, type Link } from './oracles.ts';
import { observeSections } from './sections.ts';

const optionsSchema = z.object({
	binary: z.string(),
	'source-root': z.string(),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	'input-root': z.string(),
	evidence: z.string(),
});
function escapeRegex(raw: string): string {
	return raw.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
function linkConfiguration(links: readonly Link[]): {
	rule: JsonObject;
	kind: JsonObject;
	definitions: string;
} {
	const escaped = [...new Set(links.map((link) => link.raw))].map(escapeRegex);
	const targetPatterns = [...new Map(links.map((link) => [link.raw, link])).values()].map(
		(link) => {
			const offset = link.raw.lastIndexOf(link.url);
			assert.ok(offset >= 0);
			return `${escapeRegex(link.raw.slice(0, offset))}(${escapeRegex(link.url)})${escapeRegex(link.raw.slice(offset + link.url.length))}`;
		},
	);
	return {
		definitions: `id: real-link\nlanguage: Markdown\nrule:\n  any:\n    - {kind: inline}\n    - {kind: pipe_table_cell}\n  regex: ${JSON.stringify(`(?:${escaped.join('|')})`)}\n`,
		rule: {
			name: 'original-markdown-links',
			evidence: 'link',
			severity: 'error',
			decision: {
				start: 'check',
				steps: {
					check: {
						mechanical_check: { tool: 'anchor_exists', field: 'links' },
						yes: 'pass',
						['no']: { finding: 'Inspect original broken Markdown link.' },
					},
				},
			},
		},
		kind: {
			rule: 'real-link',
			fields: {
				links: {
					from: 'target',
					regex: targetPatterns.join('|'),
					resolve: 'path',
					opening_chars: 1,
				},
			},
		},
	};
}
async function observeCollectorControl(observer: RulebookProcess, evidence: string): Promise<void> {
	const root = path.join(evidence, 'collector-control');
	filesystem.mkdirSync(root);
	const source = Buffer.from(
		'[prose](target.md#present)\n\n| Reference |\n| --- |\n| [table](target.md#present) |\n\n```md\n[table](target.md#present)\n```\n',
	);
	filesystem.writeFileSync(path.join(root, 'source.md'), source);
	filesystem.writeFileSync(path.join(root, 'target.md'), '# Present\n');
	const links: Link[] = ['prose', 'table'].map((label) => {
		const raw = `[${label}](target.md#present)`;
		const start = source.indexOf(raw);
		return {
			source: 'source.md',
			target: 'target.md',
			fragment: 'present',
			url: 'target.md#present',
			raw,
			expected: 'found',
			start,
			end: start + Buffer.byteLength(raw),
			source_sha256: createHash('sha256').update(source).digest('hex'),
		};
	});
	const fixture = linkConfiguration(links);
	writeProject(
		root,
		{ K001: fixture.rule },
		{ link: fixture.kind },
		['source.md'],
		fixture.definitions,
	);
	const configFile = path.join(root, 'sunstack.lint.json');
	const config = readObject(configFile);
	objectValue(config['decision_model'])['endpoint'] = observer.endpoint;
	writeJson(configFile, config);
	await observer.run({
		root,
		name: 'collector-check',
		arguments: ['lint', '--check'],
		expectedExit: 0,
	});
	const result = await observer.report({
		root,
		name: 'collector-links',
		arguments: ['lint', '--all', '--format', 'json'],
		expectedExit: 0,
	});
	requireLinks(result.report, links, new Map([['source.md', source]]));
}
export async function observeMarkdown(options: z.infer<typeof optionsSchema>): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	requireDebugSignature(target.binary);
	const input = path.resolve(options['input-root']);
	const evidence = path.resolve(options.evidence);
	requireOutsideInput(input, evidence);
	requireOutsideInput(target.source, evidence);
	requireInputs(input);
	filesystem.mkdirSync(evidence);
	writeJson(
		path.join(evidence, 'input-before.json'),
		repositories.map((repository) => ({
			repo: repository.repo,
			commit: repository.commit,
			files: repository.files,
		})),
	);
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	let sections = 0;
	try {
		await observeCollectorControl(observer, evidence);
		await runSequence(repositories, async (repository) => {
			const root = path.join(evidence, repository.key);
			filesystem.mkdirSync(root);
			for (const file of repository.files) {
				const destination = path.join(root, file.file);
				filesystem.mkdirSync(path.dirname(destination), { recursive: true });
				filesystem.writeFileSync(destination, requireFile(path.join(input, repository.key), file));
			}
			const fixture = linkConfiguration(repository.links);
			writeProject(
				root,
				{ K001: fixture.rule },
				{ link: fixture.kind },
				[...new Set(repository.links.map((link) => link.source))],
				fixture.definitions,
			);
			const configFile = path.join(root, 'sunstack.lint.json');
			const config = readObject(configFile);
			objectValue(config['decision_model'])['endpoint'] = observer.endpoint;
			writeJson(configFile, config);
			await observer.run({
				root,
				name: `${repository.key}-check`,
				arguments: ['lint', '--check'],
				expectedExit: 0,
			});
			const result = await observer.report({
				root,
				name: `${repository.key}-all-links`,
				arguments: ['lint', '--all', '--format', 'json'],
				expectedExit: 1,
			});
			const originals = new Map(
				[...new Set(repository.links.map((link) => link.source))].map((file) => [
					file,
					filesystem.readFileSync(path.join(root, file)),
				]),
			);
			requireLinks(result.report, repository.links, originals);
			writeJson(path.join(evidence, `${repository.key}-account.json`), repository.links);
		});
		sections = await observeSections(observer, input, evidence);
		assert.equal(observer.captures.length, 0);
	} finally {
		try {
			await observer.close();
		} finally {
			requireInputs(input);
		}
	}
	writeJson(path.join(evidence, 'pass.json'), {
		target: { ...target },
		repositories: repositories.map((repository) => ({
			repo: repository.repo,
			commit: repository.commit,
			links: repository.links.length,
			found: repository.links.filter((link) => link.expected === 'found').length,
			missing: repository.links.filter((link) => link.expected === 'missing').length,
			missing_files: repository.links.filter((link) => link.expected === 'missing_file').length,
		})),
		sections,
		model_requests: 0,
		input_rechecked: true,
		limits: [
			'Rendered GitHub samples and native/parity/signing joins require their own receipts.',
			'No provider, CI, release or acceptance claim.',
		],
	});
}
if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: {
			...Object.fromEntries(
				['binary', 'source-root', 'commit', 'sha256', 'input-root', 'evidence'].map((name) => [
					name,
					{ type: 'string' as const },
				]),
			),
			help: { type: 'boolean' },
		},
	});
	if (values.help)
		process.stdout.write(
			'Observe all 264 real Markdown fragment links and 74 sections (68 real, 6 owned), exact raw field spans and admission controls.\n--binary PATH --source-root CHECKOUT --commit FULL_SHA --sha256 DIGEST --input-root FROZEN_CORPUS --evidence NEW_DIRECTORY\nInput layout: sunstack/,tools/,controls.md; hashes and independent oracles in links.json/sections.json. Keyless CLI only.\n',
		);
	else await observeMarkdown(optionsSchema.parse(values));
}
