// Load an existing CLI configuration, edit the loaded value, and run the edit.
//
//   node examples/load-and-edit.ts <absolute-config-directory> <absolute-checked-root>
import { check, load, run } from '@sunstack/sdk';
import type { Configuration, HtmlEvidenceSettings, MarkdownEvidenceSettings } from '@sunstack/sdk';

const [directory = process.cwd(), root = process.cwd()] = process.argv.slice(2);

const loaded = await load({ directory });
if (loaded.kind !== 'loaded') {
	process.stdout.write(`load: ${loaded.kind}\n`);
	process.exit(1);
}

// Document conventions as a reusable preset: here, the 0.14.1 conventions.
const restoredConventions = {
	html: { skip_tags: ['ex-markdown'], text_attributes: { 'ex-mermaid': ['label'] } },
	markdown: { heading_anchors: 'github' },
} as const satisfies { html: HtmlEvidenceSettings; markdown: MarkdownEvidenceSettings };

// The loaded value is frozen; a changed copy is a new configuration.
const { configuration } = loaded.value;
const changed = {
	...configuration,
	limits: { ...configuration.limits, max_files: 100 },
	evidence: { ...configuration.evidence, ...restoredConventions },
} satisfies Configuration;

const admission = await check(changed);
process.stdout.write(`check: ${admission.kind}\n`);
const outcome = await run(changed, { root, scope: { kind: 'all' } });
process.stdout.write(`run: ${outcome.kind}\n`);
if (outcome.kind === 'completed') {
	for (const finding of outcome.report.findings) {
		process.stdout.write(`${finding.file}:${finding.line} ${finding.rule} ${finding.message}\n`);
	}
	// Sources name exactly what executed: unchanged files, or edited values.
	for (const source of outcome.report.sources) {
		process.stdout.write(`source ${source.origin.kind} ${source.sha256.slice(0, 12)}\n`);
	}
}
