/** Plugin-style external config against an explicitly checked, read-only real project. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import filesystem from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

import { z } from 'zod';

import { requireDebugSignature } from '../rulebook/rulebook-amendment.ts';
import {
	arrayValue,
	objectValue,
	readObject,
	runSequence,
	textValue,
	writeJson,
	type JsonObject,
} from '../rulebook/rulebook-fixture-support.ts';
import { declaredTarget, RulebookProcess } from '../rulebook/rulebook-process.ts';
import { requireNoDispatch, requireRawSpan, savedPiece } from './html-checks.ts';
import { inventoryHtml } from './html-corpus.ts';
import { htmlConfig, writeProject } from './html-fixtures.ts';

const sdkDirectory = 'docs/specs/2026-11-11-typescript-sdk';
const sdkFiles = [
	'requirements.html',
	'specification.html',
	'program-design.html',
	'verification.html',
	'index.html',
];
const wholeRule = 'id: whole\nlanguage: Html\nrule: {kind: document}\n';
const schema = z.object({
	binary: z.string(),
	'source-root': z.string(),
	commit: z.string().regex(/^[a-f0-9]{40}$/),
	sha256: z.string().regex(/^[a-f0-9]{64}$/),
	'checked-root': z.string(),
	evidence: z.string(),
});
function pageHashes(root: string): Record<string, string> {
	return Object.fromEntries(
		sdkFiles.map((file) => [
			file,
			createHash('sha256')
				.update(filesystem.readFileSync(path.join(root, sdkDirectory, file)))
				.digest('hex'),
		]),
	);
}
function entry(name: string, tool: JsonObject): JsonObject {
	return {
		name,
		evidence: 'page',
		severity: 'error',
		decision: {
			start: 'check',
			steps: {
				check: {
					mechanical_check: tool,
					yes: 'pass',
					['no']: { finding: 'Inspect checked root.' },
				},
			},
		},
	};
}
function configureLoopback(root: string, endpoint: string): void {
	const file = path.join(root, htmlConfig);
	const config = readObject(file);
	config['decision_model'] = { ...objectValue(config['decision_model']), endpoint };
	writeJson(file, config);
}
function sdkConfig(root: string): void {
	writeProject(
		root,
		{
			K001: entry('root-html-page', { tool: 'regex', field: 'body', pattern: '^<!doctype html>' }),
		},
		{ page: { rule: 'whole', fields: { body: { from: 'remainder' } } } },
		[`${sdkDirectory}/*.html`],
		wholeRule,
	);
}
function sdkForwardConfig(root: string): void {
	writeProject(
		root,
		{ K001: entry('root-fragment', { tool: 'anchor_exists', field: 'links' }) },
		{
			page: {
				rule: 'fragment',
				fields: { links: { from: 'target', regex: 'href="([^"]*#[^"]*)"', resolve: 'path' } },
			},
		},
		[`${sdkDirectory}/*.html`],
		'id: fragment\nlanguage: Html\nrule: {kind: element, regex: \'^<a\\s+[^>]*href="[^"]*#\'}\n',
	);
}
function sdkLinkExpectations(checked: string): { file: string; href: string; outcome: string }[] {
	return sdkFiles.flatMap((file) => {
		const relative = `${sdkDirectory}/${file}`;
		const source = filesystem.readFileSync(path.join(checked, relative), 'utf8');
		return inventoryHtml(source).fragmentHrefs.map((href) => {
			const [name, fragment] = href.split('#');
			const destination = name
				? path.resolve(checked, sdkDirectory, name)
				: path.join(checked, relative);
			const outside = path.relative(checked, destination).startsWith('..');
			const outcome = outside
				? 'incomplete'
				: !filesystem.existsSync(destination)
					? 'finding'
					: !destination.endsWith('.html')
						? 'incomplete'
						: inventoryHtml(filesystem.readFileSync(destination, 'utf8')).ids.includes(
									fragment ?? '',
							  )
							? 'pass'
							: 'finding';
			return { file: relative, href, outcome };
		});
	});
}
function absoluteConfig(options: {
	plugin: string;
	checked: string;
	outside: string;
	kind: 'document' | 'target';
	allowed: boolean;
}): void {
	filesystem.mkdirSync(options.checked, { recursive: true });
	const target = options.allowed ? path.join(options.checked, 'target.html') : options.outside;
	filesystem.writeFileSync(
		path.join(options.checked, 'source.html'),
		`<p data-target="${target}">R1</p>`,
	);
	filesystem.writeFileSync(path.join(options.checked, 'target.html'), '<p>R1</p>');
	const field =
		options.kind === 'document'
			? { from: 'document', path: target, rule: 'whole' }
			: { from: 'target', regex: 'data-target="([^"]+)"', resolve: 'path' };
	const tool =
		options.kind === 'document'
			? { tool: 'contains', field: 'acquired', text: 'R1' }
			: { tool: 'anchor_exists', field: 'acquired' };
	writeProject(
		options.plugin,
		{ K001: entry('absolute-root', tool) },
		{ page: { rule: 'source', fields: { acquired: field } } },
		['source.html'],
		`${wholeRule}---\nid: source\nlanguage: Html\nrule: {kind: element, regex: '^<p\\s+data-target='}\n`,
	);
}
export async function observeCheckedRoot(options: z.infer<typeof schema>): Promise<void> {
	const target = declaredTarget({
		binary: options.binary,
		source: options['source-root'],
		commit: options.commit,
		digest: options.sha256,
	});
	requireDebugSignature(target.binary);
	const checked = path.resolve(options['checked-root']);
	const evidence = path.resolve(options.evidence);
	filesystem.mkdirSync(evidence);
	const hashes = pageHashes(checked);
	const original = path.join(evidence, 'read-only-inputs');
	filesystem.mkdirSync(original);
	for (const file of sdkFiles)
		filesystem.copyFileSync(path.join(checked, sdkDirectory, file), path.join(original, file));
	writeJson(path.join(evidence, 'input-identity.json'), {
		checked_root: checked,
		sha256: hashes,
		status:
			'Real live untracked design pages; guarded before and after each child, not a committed-revision claim.',
	});
	const plugin = path.join(evidence, 'plugin');
	sdkConfig(plugin);
	const forwardPlugin = path.join(evidence, 'forward-plugin');
	sdkForwardConfig(forwardPlugin);
	const links = sdkLinkExpectations(checked);
	assert.ok(links.length > 0, 'SDK set has no actual fragment links');
	writeJson(path.join(evidence, 'sdk-fragment-oracle.json'), links);
	const outside = path.join(evidence, 'outside.html');
	filesystem.writeFileSync(outside, '<!doctype html><p>R1 OUTSIDE_SENTINEL</p>');
	const observer = new RulebookProcess(target, path.join(evidence, 'run'));
	await observer.start();
	try {
		configureLoopback(plugin, observer.endpoint);
		configureLoopback(forwardPlugin, observer.endpoint);
		await runSequence([plugin, forwardPlugin], async (root) => {
			const health = await observer.run({
				root,
				name: `check-${path.basename(root)}`,
				arguments: ['lint', '--config', htmlConfig, '--root', checked, '--check'],
				expectedExit: 0,
			});
			assert.equal(health.captures.length, 0);
			assert.deepEqual(pageHashes(checked), hashes, 'Real pages drifted during health check');
		});
		const realReport = path.join(evidence, 'sdk.prepared.json');
		assert.deepEqual(pageHashes(checked), hashes, 'Real pages drifted before run');
		const result = await observer.run({
			root: plugin,
			name: 'sdk-root',
			arguments: [
				'lint',
				'--config',
				htmlConfig,
				'--root',
				checked,
				'--prepare',
				realReport,
				...sdkFiles.map((file) => path.join(checked, sdkDirectory, file)),
			],
			expectedExit: 6,
		});
		assert.deepEqual(pageHashes(checked), hashes, 'Real pages drifted during run');
		assert.equal(result.captures.length, 0);
		const report = readObject(realReport);
		requireNoDispatch(report);
		const rows = arrayValue(report['results']).map(objectValue);
		assert.deepEqual(
			rows.map((row) => textValue(row['file'])).toSorted(),
			sdkFiles.map((file) => `${sdkDirectory}/${file}`).toSorted(),
		);
		assert.ok(rows.every((row) => row['outcome'] === 'pass'));
		const forwardFile = path.join(evidence, 'sdk-forward.prepared.json');
		const forwardExit = links.some((link) => link.outcome === 'incomplete')
			? 3
			: links.some((link) => link.outcome === 'finding')
				? 1
				: 6;
		assert.deepEqual(pageHashes(checked), hashes, 'Real pages drifted before forward check');
		const forward = await observer.run({
			root: forwardPlugin,
			name: 'sdk-forward',
			arguments: [
				'lint',
				'--config',
				htmlConfig,
				'--root',
				checked,
				'--prepare',
				forwardFile,
				...sdkFiles.map((file) => path.join(checked, sdkDirectory, file)),
			],
			expectedExit: forwardExit,
		});
		assert.deepEqual(pageHashes(checked), hashes, 'Real pages drifted during forward check');
		assert.equal(forward.captures.length, 0);
		const forwardReport = readObject(forwardFile);
		requireNoDispatch(forwardReport);
		const actualLinks = arrayValue(forwardReport['results']).map(objectValue);
		assert.equal(actualLinks.length, links.length, 'Real fragment owner count differs');
		const remaining = [...links];
		const account: JsonObject[] = [];
		for (const row of actualLinks) {
			const piece = savedPiece(forwardReport, row);
			requireRawSpan(piece, checked);
			const href = textValue(piece['text']).match(/^<a\s+[^>]*href="([^"]*#[^"]*)"/)?.[1];
			const index = remaining.findIndex((link) => link.file === row['file'] && link.href === href);
			assert.ok(index >= 0, 'Unexpected or repeated real fragment owner');
			const expected = remaining.splice(index, 1)[0];
			assert.ok(expected);
			assert.equal(row['outcome'], expected.outcome, `${expected.file}: ${expected.href}`);
			account.push({ ...expected, result_id: row['id'] ?? null });
		}
		assert.equal(remaining.length, 0);
		writeJson(path.join(evidence, 'sdk-fragment-account.json'), account);
		const refused = await observer.run({
			root: plugin,
			name: 'outside-named',
			arguments: [
				'lint',
				'--config',
				htmlConfig,
				'--root',
				checked,
				'--prepare',
				path.join(evidence, 'refused.json'),
				outside,
			],
			expectedExit: 2,
		});
		assert.deepEqual(pageHashes(checked), hashes, 'Real pages drifted during refusal');
		const diagnostic = Buffer.concat([refused.stdout, refused.stderr]).toString('utf8');
		assert.ok(
			diagnostic.includes(outside) && diagnostic.includes(checked),
			'Outside-root diagnostic must name both path and root',
		);
		await runSequence(['document', 'target'] as const, async (kind) => {
			await runSequence([true, false], async (allowed) => {
				const name = `${kind}-${allowed ? 'inside' : 'outside'}`;
				const root = path.join(evidence, `${name}-plugin`);
				const content = path.join(evidence, `${name}-project`);
				absoluteConfig({ plugin: root, checked: content, outside, kind, allowed });
				configureLoopback(root, observer.endpoint);
				const health = await observer.run({
					root,
					name: `check-${name}`,
					arguments: ['lint', '--config', htmlConfig, '--root', content, '--check'],
					expectedExit: 0,
				});
				assert.equal(health.captures.length, 0);
				const saved = path.join(evidence, `${name}.prepared.json`);
				const child = await observer.run({
					root,
					name,
					arguments: [
						'lint',
						'--config',
						htmlConfig,
						'--root',
						content,
						'--prepare',
						saved,
						path.join(content, 'source.html'),
					],
					expectedExit: allowed ? 6 : 3,
				});
				assert.equal(child.captures.length, 0);
				const actual = readObject(saved);
				requireNoDispatch(actual);
				const results = arrayValue(actual['results']).map(objectValue);
				assert.equal(results.length, 1);
				assert.equal(results[0]?.['outcome'], allowed ? 'pass' : 'incomplete');
				if (!allowed)
					assert.match(
						JSON.stringify(actual['piece_evidence']),
						/outside.?root/i,
						'Outside acquisition was not identified as such',
					);
			});
		});
		writeJson(path.join(evidence, 'pass.json'), {
			target,
			checked_root: checked,
			input_sha256: hashes,
			real_pages: 5,
			real_fragment_hrefs: links.length,
			real_fragment_outcomes: links,
			outside_named_exit: 2,
			absolute_controls: 4,
			limits: [
				'Live design pages were byte-guarded; no committed-doc revision claim.',
				'Native joins separately cover --since and lint cases root behavior.',
				'No provider or release acceptance.',
			],
		});
	} finally {
		await observer.close();
	}
}
if (process.argv[1] === import.meta.filename) {
	const { values } = parseArgs({
		options: {
			binary: { type: 'string' },
			'source-root': { type: 'string' },
			commit: { type: 'string' },
			sha256: { type: 'string' },
			'checked-root': { type: 'string' },
			evidence: { type: 'string' },
			help: { type: 'boolean' },
		},
	});
	if (values.help)
		process.stdout.write(
			'Observe external plugin config with --root on the five real SDK design pages, outside-root refusal and absolute document/target controls.\n--binary PATH --source-root CHECKOUT --commit FULL_SHA --sha256 DIGEST --checked-root SDK_CHECKOUT --evidence NEW_DIRECTORY\nReads real pages without modifying them; stops on page drift. Keyless prepare only.\n',
		);
	else await observeCheckedRoot(schema.parse(values));
}
