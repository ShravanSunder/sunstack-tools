import rehypeStringify from 'rehype-stringify';
import remarkDirective from 'remark-directive';
import remarkGfm from 'remark-gfm';
import remarkParse from 'remark-parse';
import remarkRehype from 'remark-rehype';
import { unified } from 'unified';

import { escapeHtml } from './html-document.ts';

export async function renderMarkdownDirectives(
	sourceText: string,
	diagrams: Readonly<Record<string, string>>,
): Promise<string> {
	const parser = unified().use(remarkParse).use(remarkGfm).use(remarkDirective);
	const markdownCompiler = unified()
		.use(remarkParse)
		.use(remarkGfm)
		.use(remarkRehype)
		.use(rehypeStringify);
	const sourceTree = parser.parse(sourceText);
	const blocks = await Promise.all(
		sourceTree.children.map(async (sourceNode): Promise<string> => {
			if (sourceNode.type === 'html')
				throw new Error('Raw HTML is not part of this Markdown experiment.');
			if (sourceNode.type === 'leafDirective') {
				if (sourceNode.name !== 'diagram')
					throw new Error(`Unsupported directive: ${sourceNode.name}`);
				const diagramSource = sourceNode.attributes?.['src'];
				if (!diagramSource || !Object.hasOwn(diagrams, diagramSource))
					throw new Error(`Unknown diagram reference: ${diagramSource}`);
				return `<figure>${diagrams[diagramSource]}<figcaption>${escapeHtml(sourceNode.attributes?.['title'] ?? diagramSource)}</figcaption></figure>`;
			} else if (sourceNode.type === 'containerDirective') {
				if (!['decision', 'timeline', 'risk'].includes(sourceNode.name))
					throw new Error(`Unsupported directive: ${sourceNode.name}`);
				const transformedTree = await markdownCompiler.run({
					type: 'root',
					children: sourceNode.children,
				});
				const childHtml = markdownCompiler.stringify(transformedTree);
				const tagName = sourceNode.name === 'timeline' ? 'section' : 'aside';
				return `<${tagName} class="${sourceNode.name}"><strong>${escapeHtml(sourceNode.name)}</strong>${childHtml}</${tagName}>`;
			} else {
				const transformedTree = await markdownCompiler.run({
					type: 'root',
					children: [sourceNode],
				});
				return markdownCompiler.stringify(transformedTree);
			}
		}),
	);
	return blocks.join('\n');
}
