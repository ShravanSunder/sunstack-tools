import { inspect as formatValue } from 'node:util';

// Author a complete configuration as TypeScript values: no definition files.
//
//   node examples/inline-configuration.ts <absolute-checked-root> [endpoint]
//
// `check` needs no credential. `run` uses the ordinary credential route
// (OPENROUTER_API_KEY or the OS store) and the configured decision endpoint.
import { check, run } from '@sunstack/sdk';
import type { Configuration, RunOutcome } from '@sunstack/sdk';

const [root = process.cwd(), endpoint] = process.argv.slice(2);

const configuration = {
	id: 'instruction-standards',
	base_directory: import.meta.dirname,
	marker_prefix: 'K',
	check: { files: ['skills/**/SKILL.md'] },
	decision_model: { model: 'jev-latest', ...(endpoint === undefined ? {} : { endpoint }) },
	limits: { max_file_bytes: 1_048_576n },
	evidence: {
		kind: 'inline',
		utilities: {},
		matchers: {
			paragraph: { language: 'Markdown', rule: { kind: 'paragraph' } },
		},
		kinds: {
			paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } },
		},
	},
	filters: {
		purpose: {
			per: 'skills/{name}',
			reads: { file: 'SKILL.md', kind: 'paragraph', fields: ['body'] },
			labels: ['eligible', 'blocked'],
			questions: {
				purpose: {
					type: 'choice',
					instructions: { check: 'body', task: 'Classify whether this is reusable guidance.' },
					criteria: { portable: 'Reusable guidance.', local: 'One repository procedure.' },
				},
			},
			decision: {
				start: 'purpose',
				steps: {
					purpose: {
						ask: 'purpose',
						min_probability: 0.7,
						routes: { portable: { label: 'eligible' }, local: { label: 'blocked' } },
						uncertain: 'unsure',
					},
				},
			},
		},
	},
	rulebooks: [
		{
			kind: 'inline',
			id: 'ownership-standards',
			markdown: '# Standards\n\nEach instruction names its responsible actor. <!-- K001 -->\n',
			entries: {
				K001: {
					name: 'instruction_actor',
					evidence: 'paragraph',
					severity: 'warn',
					only_when: { purpose: ['eligible'] },
					questions: {
						actor: {
							type: 'noul',
							instructions: { check: 'body', against: 'standard', task: 'Is the actor named?' },
						},
					},
					decision: {
						start: 'actor',
						steps: {
							actor: {
								ask: 'actor',
								yes_at_or_above: 0.8,
								no_at_or_below: 0.2,
								yes: 'pass',
								no: { finding: 'Name the actor.', at: 'body' },
								uncertain: 'uncertain',
							},
						},
					},
				},
			},
		},
	],
} satisfies Configuration;

function summarize(outcome: RunOutcome): string {
	switch (outcome.kind) {
		case 'completed':
			return `${outcome.report.status.kind}: ${outcome.report.findings.length} finding(s)`;
		case 'invalid':
		case 'unavailable':
			return `${outcome.kind}: ${outcome.diagnostics.map((diagnostic) => diagnostic.code).join(', ')}`;
		case 'incomplete':
			return `incomplete: ${outcome.report.incomplete.length} gap(s)`;
		case 'cancelled':
			return 'cancelled';
		case 'failed':
			return `failed (${outcome.failure.reason}, effects ${outcome.failure.effects})`;
		default:
			return assertNever(outcome);
	}
}

function assertNever(value: never): never {
	throw new Error(`Unhandled outcome: ${JSON.stringify(value)}`);
}

const admission = await check(configuration); // no model request
process.stdout.write(`check: ${admission.kind}\n`);
if (admission.kind === 'valid') {
	const outcome = await run(configuration, { root, scope: { kind: 'all' } });
	process.stdout.write(`run: ${summarize(outcome)}\n`);
	// This asking/filter recipe expects clean findings and actual evaluated input.
	process.exitCode =
		outcome.kind === 'completed' &&
		outcome.report.status.kind === 'clean' &&
		outcome.report.results.length > 0
			? 0
			: 1;
} else {
	process.stderr.write(`${formatValue(admission, { depth: null })}\n`);
	process.exitCode = 1;
}
