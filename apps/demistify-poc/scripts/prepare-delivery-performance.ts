/** Bounded experiment: prepare the same review three ways; no runtime or authoring-API change. */
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';

import { renderMarkdownContent } from '@sunstack/demistify/markdown-content';
import { prepareIslandContent } from '@sunstack/demistify/packaging';
import { parse, parseFragment, serialize } from 'parse5';
import type { DefaultTreeAdapterTypes } from 'parse5';

const markdownOwners = new Set(['ex-document', 'ex-card', 'ex-callout']);

function isTextNode(node: DefaultTreeAdapterTypes.Node): node is DefaultTreeAdapterTypes.TextNode {
	return node.nodeName === '#text';
}

/** Match bare-text adoption only for the owners this fixed review contains. */
function prepareOwnedMarkdown(node: DefaultTreeAdapterTypes.Node): void {
	if (!('childNodes' in node)) return;
	for (const child of node.childNodes) prepareOwnedMarkdown(child);
	if (!('tagName' in node) || !markdownOwners.has(node.tagName)) return;
	const children: DefaultTreeAdapterTypes.ChildNode[] = [];
	let pending: DefaultTreeAdapterTypes.TextNode[] = [];
	const flushText = (): void => {
		if (pending.some((text): boolean => text.value.trim().length > 0)) {
			const sourceText = pending.map((text): string => text.value).join('');
			const fragment = parseFragment(
				`<ex-markdown prepared>${renderMarkdownContent(sourceText)}</ex-markdown>`,
			);
			for (const child of fragment.childNodes) {
				child.parentNode = node;
				children.push(child);
			}
		} else children.push(...pending);
		pending = [];
	};
	for (const child of node.childNodes) {
		if (isTextNode(child)) pending.push(child);
		else {
			flushText();
			children.push(child);
		}
	}
	flushText();
	node.childNodes = children;
}

const outputArgument = process.argv[2];
if (!outputArgument)
	throw new Error(
		'Usage: node apps/demistify-poc/scripts/prepare-delivery-performance.ts OUTPUT_DIRECTORY',
	);
const outputDirectory = resolve(outputArgument);
const sourceHtml = await readFile(
	new URL(
		'../experiments/agent-written/round-15-presentation-skill/luna-corrected.html',
		import.meta.url,
	),
	'utf8',
);
if (!sourceHtml.includes('<ex-document>')) throw new Error('The fixed review source changed.');
const navigationHtml = sourceHtml.replace(
	'<ex-document>',
	'<ex-nav for="ex-content"></ex-nav>\n<ex-document id="ex-content">',
);
const preparationStart = performance.now();
// Prepare inert diagram sources first: this helper expects author input, not prepared Markdown.
const preparedDocument = parse(prepareIslandContent(navigationHtml, {}, true));
prepareOwnedMarkdown(preparedDocument);
const preparedHtml = serialize(preparedDocument);
const preparationMs = performance.now() - preparationStart;
await mkdir(outputDirectory, { recursive: true });
const variants = [
	{ file: 'baseline.html', html: sourceHtml, generated: false },
	{ file: 'explicit-outline.html', html: navigationHtml, generated: false },
	{ file: 'prepared.html', html: preparedHtml, generated: true },
];
await Promise.all(
	variants.map(async (variant): Promise<void> => {
		await writeFile(resolve(outputDirectory, variant.file), variant.html);
	}),
);
const manifest = {
	input: 'round-15-presentation-skill/luna-corrected.html',
	inputSha256: createHash('sha256').update(sourceHtml).digest('hex'),
	preparationMs,
	variants: variants.map((variant) => ({
		file: variant.file,
		generated: variant.generated,
		bytes: Buffer.byteLength(variant.html),
		sha256: createHash('sha256').update(variant.html).digest('hex'),
	})),
	limits:
		'One fixed review and its existing component owners. Generated prepared markup is not authoring vocabulary. This does not prepare arbitrary page assets, explorer data or every island.',
};
await writeFile(
	resolve(outputDirectory, 'preparation.json'),
	`${JSON.stringify(manifest, null, 2)}\n`,
);
process.stdout.write(`${JSON.stringify(manifest, null, 2)}\n`);
