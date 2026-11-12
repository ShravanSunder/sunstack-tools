// A keyless run: a mechanical rule needs no model, so no credential is used.
//
//   node examples/mechanical-run.ts <absolute-checked-root>
import { check, contractInfo, run } from '@sunstack/sdk';
import type { Configuration } from '@sunstack/sdk';

const [root = process.cwd()] = process.argv.slice(2);

const contract = await contractInfo();
if (contract.kind === 'failed') {
	process.stdout.write(`addon: ${contract.failure.reason}: ${contract.failure.message}\n`);
	process.exit(1);
}
process.stdout.write(
	`contract: release ${contract.contract.sdk_release} revision ${contract.contract.contract_revision} target ${contract.contract.target}\n`,
);

const configuration = {
	id: 'description-standards',
	base_directory: import.meta.dirname,
	marker_prefix: 'K',
	check: { files: ['docs/*.md'] },
	decision_model: { model: 'jev-latest' },
	evidence: {
		kind: 'inline',
		utilities: {},
		matchers: { skill: { language: 'Markdown', rule: { matches: 'frontmatter' } } },
		kinds: {
			skill: { rule: 'skill', fields: { description: { from: 'yaml', key: 'description' } } },
		},
	},
	rulebooks: [
		{
			kind: 'inline',
			id: 'description-rules',
			markdown: 'Description must begin with Use when. <!-- K001 -->\n',
			entries: {
				K001: {
					name: 'description',
					evidence: 'skill',
					severity: 'error',
					decision: {
						start: 'starts',
						steps: {
							starts: {
								mechanical_check: { tool: 'regex', field: 'description', pattern: '^Use when' },
								yes: 'pass',
								no: { finding: 'Description must begin with Use when.', at: 'description' },
							},
						},
					},
				},
			},
		},
	],
} satisfies Configuration;

const admission = await check(configuration);
process.stdout.write(`check: ${admission.kind}\n`);
const outcome = await run(configuration, { root, scope: { kind: 'all' } });
if (outcome.kind === 'completed') {
	process.stdout.write(`run: ${outcome.report.status.kind} ${outcome.report.findings.length}\n`);
	for (const finding of outcome.report.findings) {
		process.stdout.write(`${finding.file}:${finding.line} ${finding.message}\n`);
	}
} else {
	process.stdout.write(`run: ${outcome.kind}\n`);
}
