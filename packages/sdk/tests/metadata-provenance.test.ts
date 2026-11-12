// Unchanged file metadata keeps its file provenance after the JavaScript round trip,
// even where JSON numbers lose the integral-float spelling (2.0 arrives as 2).
import assert from 'node:assert/strict';
import { test } from 'node:test';

import type { AstMatcher, Configuration, LoadedConfiguration, RunOutcome } from '../src/index.ts';
import { check, load, run } from '../src/index.ts';
import { ownedDirectory, purposeFilter, writeFiles } from './support/sdk-fixtures.ts';

delete process.env['OPENROUTER_API_KEY'];
process.env['SUNSTACK_CREDENTIAL_STORE'] = 'off';

/** Authored JSON data in each definition that carries it, as raw source text. */
interface AuthoredData {
	/** The matcher's YAML `metadata`. */
	readonly metadata: string;
	/** The rule entry's JSON `reference`. */
	readonly entryReference: string;
	/** The root filter's JSON `reference`; a filter asks the model, so runs leave it out. */
	readonly filterReference?: string;
}

/** A keyless file project whose matcher, rule entry and root filter carry the given data. */
function writeProject(directory: string, data: AuthoredData): void {
	const filter =
		data.filterReference === undefined
			? '{}'
			: JSON.stringify(purposeFilter).replace(/\}$/u, `,"reference":${data.filterReference}}`);
	const config = JSON.stringify({
		rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
		marker_prefix: 'K',
		check: { files: ['docs/*.md'] },
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
		},
		decision_model: { model: 'jev-latest' },
		filters: 'FILTERS',
	}).replace('"FILTERS"', data.filterReference === undefined ? filter : `{"purpose":${filter}}`);
	const entry = JSON.stringify({
		K001: {
			name: 'body',
			evidence: 'paragraph',
			severity: 'error',
			decision: {
				start: 'has',
				steps: {
					has: {
						mechanical_check: { tool: 'complete', field: 'body' },
						yes: 'pass',
						no: { finding: 'Empty.' },
					},
				},
			},
			reference: 'REFERENCE',
		},
	}).replace('"REFERENCE"', data.entryReference);
	writeFiles(directory, {
		'sunstack.lint.json': config,
		'rulebook.md': 'Each paragraph has a body. <!-- K001 -->\n',
		'rules.json': entry,
		'evidence/sgconfig.yml': 'ruleDirs: [rules]\n',
		'evidence/rules/paragraph.yml': `id: paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\nmetadata: ${data.metadata}\n`,
		'docs/one.md': '# One\n\nA paragraph.\n',
	});
}

async function loaded(directory: string): Promise<LoadedConfiguration> {
	const outcome = await load({ directory });
	if (outcome.kind !== 'loaded') throw new Error(`load: ${outcome.kind}`);
	return outcome.value;
}

/** The origin kind of each source a check admits, and the matcher source's content. */
async function admitted(
	configuration: LoadedConfiguration | Configuration,
): Promise<{ origins: string[]; matcher: string | undefined }> {
	const outcome = await check(configuration);
	assert.equal(outcome.kind, 'valid');
	if (outcome.kind !== 'valid') return { origins: [], matcher: undefined };
	return {
		origins: outcome.sources.map((source) => source.origin.kind),
		matcher: outcome.sources.find((source) =>
			source.origin.kind === 'file'
				? source.origin.path.endsWith('paragraph.yml')
				: source.origin.kind === 'edited' &&
					source.origin.id.endsWith('/evidence/matchers/paragraph'),
		)?.content,
	};
}

function completedOrigins(outcome: RunOutcome): string[] {
	assert.equal(outcome.kind, 'completed');
	return outcome.kind === 'completed'
		? outcome.report.sources.map((source) => source.origin.kind)
		: [];
}

/** A copy of the loaded configuration with different matcher metadata. */
function withMetadata(
	held: LoadedConfiguration,
	metadata: NonNullable<AstMatcher['metadata']>,
): Configuration {
	const { configuration } = held;
	const matcher =
		configuration.evidence.kind === 'captured'
			? configuration.evidence.matchers['paragraph']
			: undefined;
	if (configuration.evidence.kind !== 'captured' || matcher === undefined)
		throw new Error('captured matcher');
	return {
		...configuration,
		evidence: {
			...configuration.evidence,
			matchers: {
				paragraph: { ...matcher, value: { ...matcher.value, metadata } },
			},
		},
	};
}

/** Load, check and (without a filter) run one project; every source must stay its file. */
async function assertFileProvenance(name: string, data: AuthoredData): Promise<void> {
	const directory = ownedDirectory(`metadata-${name.replaceAll(/[^a-z]+/gu, '-')}`);
	try {
		writeProject(directory.path, data);
		const held = await loaded(directory.path);
		const { origins, matcher } = await admitted(held);
		assert.deepEqual(origins, ['file', 'file', 'file', 'file'], name);
		// The receipt is the original file text, not a re-serialized value.
		assert.ok(matcher?.includes(`metadata: ${data.metadata}`), name);
		if (data.filterReference === undefined) {
			const root = { root: directory.path, scope: { kind: 'all' } as const };
			assert.deepEqual(
				completedOrigins(await run(held, root)),
				['file', 'file', 'file', 'file'],
				name,
			);
		}
	} finally {
		directory.remove();
	}
}

await test('unchanged integral-float, fractional and integer metadata keep file provenance', async () => {
	const cases: readonly (readonly [string, AuthoredData])[] = [
		[
			'integral float',
			{
				metadata: '{owner: docs, weight: 2.0, nested: {ratio: 1.0, list: [3.0, 1.5, -4.0]}}',
				entryReference: '{"threshold": 2.0, "steps": [1.0, 0.5]}',
			},
		],
		[
			'fractional',
			{ metadata: '{owner: docs, weight: 1.5}', entryReference: '{"threshold": 1.5}' },
		],
		['integer', { metadata: '{owner: docs, weight: 2}', entryReference: '{"threshold": 2}' }],
		[
			'integral float in a root filter (check only)',
			{
				metadata: '{owner: docs}',
				entryReference: '{}',
				filterReference: '{"cap": 1.0, "levels": [2.0, 0.5]}',
			},
		],
	];
	await Promise.all(cases.map(async ([name, data]) => assertFileProvenance(name, data)));
});

await test('a real numeric or text change to integral-float metadata is an edit with the supplied value', async () => {
	const directory = ownedDirectory('metadata-edits');
	try {
		writeProject(directory.path, {
			metadata: '{owner: docs, weight: 2.0}',
			entryReference: '{"threshold": 2.0}',
		});
		const held = await loaded(directory.path);
		const edits = [
			['number', { owner: 'docs', weight: 3 }, '"weight":3'],
			['text', { owner: 'ops', weight: 2 }, '"owner":"ops"'],
		] as const;
		await Promise.all(
			edits.map(async ([name, metadata, token]) => {
				const edited = withMetadata(held, metadata);
				const { origins, matcher } = await admitted(edited);
				assert.deepEqual(origins, ['file', 'file', 'file', 'edited'], name);
				assert.ok(matcher?.includes(token), `${name}: the edited source holds the supplied value`);
				const completed = await run(edited, { root: directory.path, scope: { kind: 'all' } });
				assert.deepEqual(completedOrigins(completed), ['file', 'file', 'file', 'edited'], name);
			}),
		);
	} finally {
		directory.remove();
	}
});
