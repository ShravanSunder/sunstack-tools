/**
 * Static checks for an explainer page, run in Node without a browser: vocabulary, attribute values,
 * id references, nesting, island sources, diagrams and styling. Each finding names its line and,
 * when the repair is mechanical, carries the text edit that fixes it.
 */
import { dirname, join } from 'node:path';

import { type DefaultTreeAdapterMap, parse } from 'parse5';

import { headingSlug } from '../core/heading-fragments.ts';
import { parseMarkdownSource } from '../core/markdown-source.ts';
import { normalizeIslandText } from '../core/source-text.ts';
import { parseGraphDocument } from '../explorer/graph-document.ts';
import { parseCode } from '../render/code-lines.ts';
import { renderThemedDiagram } from '../render/diagram.ts';
import { hasGraphEdges } from '../render/graph-source.ts';
import { renderMarkdownContent } from '../render/markdown-content.ts';
import { checkAttributes, closest, taughtElements, vocabulary } from '../vocabulary.ts';

type Element = DefaultTreeAdapterMap['element'];
type ParentNode = DefaultTreeAdapterMap['parentNode'];

export interface TextEdit {
	readonly start: number;
	readonly end: number;
	readonly text: string;
}
export interface Finding {
	readonly line: number;
	readonly column: number;
	readonly severity: 'error' | 'warning';
	readonly rule: string;
	readonly message: string;
	readonly edit?: TextEdit;
}

const elementAliases: Readonly<Record<string, string>> = {
	'ex-diagram': 'ex-mermaid',
	'ex-chart': 'ex-mermaid (xychart-beta)',
	'ex-graph': 'ex-mermaid',
	'ex-section': 'ex-grid or a heading',
	'ex-row': 'ex-grid',
	'ex-columns': 'ex-grid',
	'ex-box': 'ex-card',
	'ex-panel': 'ex-card',
	'ex-warning': 'ex-callout tone="risk"',
	'ex-tab': '<section label> inside ex-tabs',
	'ex-steps': 'ex-presentation',
	'ex-connector': 'ex-arrow',
	'ex-link': 'ex-arrow',
};
/** Elements whose bare text is Markdown. */
const markdownHosts = new Set([
	'ex-document',
	'section',
	'ex-card',
	'ex-callout',
	'ex-tree',
	'ex-region',
	'details',
]);
/** CSS the author may write: layout only; the library owns colour, borders, type and shadows. */
const layoutProperty =
	/^(display|grid|gap|row-gap|column-gap|flex|order|align|justify|place|position|inset|top|right|bottom|left|width|height|min-|max-|margin|padding|overflow|z-index|aspect-ratio|box-sizing|container)/;

function isElement(node: DefaultTreeAdapterMap['node']): node is Element {
	return 'tagName' in node;
}

function* walk(parent: ParentNode): Generator<Element> {
	for (const child of parent.childNodes) {
		if (!isElement(child)) continue;
		yield child;
		if (child.tagName === 'template' && 'content' in child) yield* walk(child.content);
		else yield* walk(child);
	}
}

function attributeMap(element: Element): Map<string, string> {
	return new Map(
		element.attrs.map((attribute): [string, string] => [attribute.name, attribute.value]),
	);
}

function parentElement(element: Element): Element | undefined {
	const parent = element.parentNode;
	return parent && isElement(parent) ? parent : undefined;
}

function isDedicatedGalleryDocument(gallery: Element, documentCount: number): boolean {
	const parent = parentElement(gallery);
	if (parent?.tagName !== 'ex-document' || documentCount !== 1) return false;
	const content = parent.childNodes.filter((child): boolean => {
		if ('value' in child) return child.value.trim().length > 0;
		return (
			isElement(child) &&
			!['script', 'template', 'style'].includes(child.tagName) &&
			!attributeMap(child).has('hidden')
		);
	});
	if (content.length === 1) return content[0] === gallery;
	const heading = content[0];
	return (
		content.length === 2 &&
		!!heading &&
		isElement(heading) &&
		heading.tagName === 'h1' &&
		content[1] === gallery
	);
}

function hasAncestor(element: Element, names: readonly string[]): boolean {
	for (let parent = parentElement(element); parent; parent = parentElement(parent))
		if (names.includes(parent.tagName)) return true;
	return false;
}

function isTemplateElement(element: Element): element is DefaultTreeAdapterMap['template'] {
	return element.tagName === 'template';
}

function textOf(element: Element): string {
	const children = isTemplateElement(element) ? element.content.childNodes : element.childNodes;
	return children
		.map((child): string =>
			'value' in child && typeof child.value === 'string'
				? child.value
				: isElement(child)
					? textOf(child)
					: '',
		)
		.join('');
}

function cssProblems(css: string): string[] {
	return css
		.replace(/\/\*[\s\S]*?\*\//g, '')
		.split(/[;{}]/)
		.map((declaration): string => declaration.split(':')[0]?.trim().toLowerCase() ?? '')
		.filter(
			(property): boolean =>
				/^[a-z-]+$/.test(property) && !property.startsWith('--') && !layoutProperty.test(property),
		);
}

export interface CheckOptions {
	/** Render each diagram with the diagram engine to catch syntax the engine rejects. */
	readonly diagrams?: boolean;
	/** Reads a diagram file named by `src`, relative to the page. */
	readonly readSource?: (path: string) => Promise<string | undefined>;
}

export async function checkHtml(source: string, options: CheckOptions = {}): Promise<Finding[]> {
	const findings: Finding[] = [];
	const document = parse(source, { sourceCodeLocationInfo: true });
	const elements = Array.from(walk(document));
	const inertTemplateElements = new Set(
		elements
			.filter(isTemplateElement)
			.flatMap((template): Element[] => Array.from(walk(template.content))),
	);
	const documentCount = elements.filter(
		(element): boolean => element.tagName === 'ex-document' && !inertTemplateElements.has(element),
	).length;
	const report = (
		element: Element,
		severity: Finding['severity'],
		rule: string,
		message: string,
		attribute?: string,
		edit?: TextEdit,
	): void => {
		const location = element.sourceCodeLocation;
		const place =
			(attribute ? location?.attrs?.[attribute] : undefined) ?? location?.startTag ?? location;
		findings.push({
			line: place?.startLine ?? 1,
			column: place?.startCol ?? 1,
			severity,
			rule,
			message,
			...(edit ? { edit } : {}),
		});
	};

	// Ids outside SVG, and words in diagram sources (route steps may target diagram nodes).
	const ids = new Map<string, Element>();
	const diagramWords = new Map<string, ReadonlySet<string>>();
	for (const element of elements) {
		if (element.tagName === 'ex-mermaid') {
			const ownerId = attributeMap(element).get('id');
			if (ownerId)
				diagramWords.set(ownerId, new Set(textOf(element).match(/[A-Za-z_][\w-]*/g) ?? []));
		}
		const id = attributeMap(element).get('id');
		if (!id || element.namespaceURI.endsWith('svg') || hasAncestor(element, ['svg'])) continue;
		if (ids.has(id))
			report(element, 'error', 'duplicate-id', `Duplicate id "${id}"; ids must be unique.`, 'id');
		else ids.set(id, element);
	}

	const markdownTargets = async (
		pageElements: readonly Element[],
		relativeTo = '',
	): Promise<ReadonlySet<string>> => {
		const targets = new Set<string>();
		const addDiagramTargets = (diagram: Element, ownerId: string): void => {
			targets.add(ownerId);
			for (const word of textOf(diagram).match(/[A-Za-z_][\w-]*/g) ?? [])
				targets.add(`${ownerId}--${word}`);
		};
		for (const diagram of pageElements.filter(
			(element): boolean => element.tagName === 'ex-mermaid',
		)) {
			const ownerId = attributeMap(diagram).get('id');
			if (ownerId) addDiagramTargets(diagram, ownerId);
		}
		const owners = pageElements.filter((element): boolean => element.tagName === 'ex-markdown');
		const sources = await Promise.all(
			owners.map((owner): Promise<string | undefined> => {
				const reference = attributeMap(owner).get('src');
				return reference && options.readSource
					? options.readSource(join(relativeTo, reference))
					: Promise.resolve(undefined);
			}),
		);
		for (const [index, owner] of owners.entries()) {
			const attrs = attributeMap(owner);
			const prefix = attrs.get('id');
			if (!prefix) continue;
			const sourceElement = owner.childNodes
				.filter(isElement)
				.find((child): boolean => child.tagName === 'script');
			const external = sources[index];
			const content =
				external ??
				(sourceElement && attributeMap(sourceElement).has('data-ex-markdown-source')
					? parseMarkdownSource(textOf(sourceElement)).text
					: sourceElement
						? textOf(sourceElement)
						: textOf(owner));
			const rendered = Array.from(walk(parse(renderMarkdownContent(content))));
			const diagrams = rendered.filter((element): boolean => element.tagName === 'ex-mermaid');
			for (const [diagramIndex, diagram] of diagrams.entries())
				addDiagramTargets(diagram, `${prefix}--diagram-${diagramIndex + 1}`);
			const seen = new Map<string, number>();
			for (const heading of rendered.filter((element): boolean =>
				/^h[1-6]$/.test(element.tagName),
			)) {
				const slug = headingSlug(textOf(heading));
				const occurrence = (seen.get(slug) ?? 0) + 1;
				seen.set(slug, occurrence);
				targets.add(`${prefix}--${slug}${occurrence > 1 ? `-${occurrence}` : ''}`);
			}
		}
		return targets;
	};
	for (const target of await markdownTargets(elements))
		if (!ids.has(target)) {
			const owner = elements.find((element): boolean => element.tagName === 'ex-markdown');
			if (owner) ids.set(target, owner);
		}

	const engine =
		options.diagrams !== false &&
		elements.some((element): boolean => element.tagName === 'ex-mermaid')
			? await import('beautiful-mermaid')
			: undefined;
	const otherPages = new Map<string, Set<string> | undefined>();
	const idsOnPage = async (file: string): Promise<Set<string> | undefined> => {
		if (!otherPages.has(file)) {
			const text = await options.readSource?.(file);
			otherPages.set(
				file,
				text === undefined
					? undefined
					: new Set(
							Array.from(text.matchAll(/\sid="([^"]+)"/g), (match): string => match[1] ?? ''),
						),
			);
		}
		const targets = otherPages.get(file);
		if (targets && options.readSource) {
			const referencedHtml = await options.readSource(file);
			if (referencedHtml)
				for (const target of await markdownTargets(
					Array.from(walk(parse(referencedHtml))),
					dirname(file),
				))
					targets.add(target);
		}
		return targets;
	};
	let loadsTheme = false;
	let loadsRuntime = false;
	for (const element of elements) {
		const name = element.tagName;
		const attributes = attributeMap(element);
		if (
			name === 'ex-gallery' &&
			!inertTemplateElements.has(element) &&
			!isDedicatedGalleryDocument(element, documentCount)
		)
			report(
				element,
				'error',
				'gallery-page',
				'A Gallery needs a dedicated page: one <ex-gallery> directly in <ex-document>, with only an optional preceding <h1>. Move prose to another page and link here; use an ordinary <figure> for an inline picture.',
			);
		if (name === 'link' && /theme\.css(\?|$)/.test(attributes.get('href') ?? '')) loadsTheme = true;
		if (name === 'script' && /runtime\.js(\?|$)/.test(attributes.get('src') ?? ''))
			loadsRuntime = true;
		if (name === 'style')
			for (const property of new Set(cssProblems(textOf(element))))
				report(
					element,
					'warning',
					'styling',
					`<style> sets ${property}; the library owns colour, borders, type and shadows. Keep only layout CSS.`,
				);
		const inlineStyle = attributes.get('style');
		if (inlineStyle)
			for (const property of new Set(cssProblems(inlineStyle)))
				report(
					element,
					'warning',
					'styling',
					`style sets ${property}; keep only layout CSS (grid, flex, gaps, positions, sizes).`,
					'style',
				);
		const parent = parentElement(element);
		for (const problem of checkAttributes(name, attributes, parent?.tagName)) {
			const location = element.sourceCodeLocation?.attrs?.[problem.attribute];
			const edit =
				problem.fix && location
					? {
							start: location.startOffset,
							end: location.endOffset,
							text: `${problem.fix.name ?? problem.attribute}="${problem.fix.value ?? attributes.get(problem.attribute) ?? ''}"`,
						}
					: undefined;
			report(
				element,
				problem.severity ?? 'error',
				'attribute',
				problem.message,
				location ? problem.attribute : undefined,
				edit,
			);
		}
		if (!name.startsWith('ex-')) continue;

		const spec = vocabulary[name];
		if (!spec) {
			const meant = elementAliases[name] ?? closest(name, taughtElements);
			report(
				element,
				'error',
				'unknown-element',
				`Unknown element <${name}>.${meant ? ` Use ${meant}.` : ` Known: ${taughtElements.join(', ')}.`}`,
			);
			continue;
		}

		if (spec.within && !hasAncestor(element, spec.within))
			report(
				element,
				'error',
				'nesting',
				`<${name}> belongs inside <${spec.within.join('> or <')}>.`,
			);

		// References to other elements.
		for (const [attribute, attributeSpec] of Object.entries(spec.attributes)) {
			const value = attributes.get(attribute);
			if (!value || (attributeSpec.type.kind !== 'id' && attributeSpec.type.kind !== 'ids'))
				continue;
			for (const target of value.split(/\s+/).filter(Boolean)) {
				// A tour step on another page: the file must exist next to this one, with that id.
				if ((name === 'ex-step' || name === 'ex-presentation') && /\.html?($|[?#])/i.test(target)) {
					const [file = '', hash = ''] = target.split('#');
					// oxlint-disable-next-line no-await-in-loop -- other pages are read once each, in page order.
					const otherIds = options.readSource ? await idsOnPage(file) : new Set<string>([hash]);
					if (!otherIds)
						report(
							element,
							'error',
							'missing-page',
							`${attribute}="${target}": ${file} is not a page next to this one.`,
							attribute,
						);
					else if (hash && !otherIds.has(decodeURIComponent(hash)))
						report(
							element,
							'error',
							'missing-id',
							`${attribute}="${target}": ${file} has no element with id "${hash}".`,
							attribute,
						);
					continue;
				}
				if (
					ids.has(target) ||
					(name === 'ex-step' &&
						target.includes('--') &&
						diagramWords
							.get(target.slice(0, target.lastIndexOf('--')))
							?.has(target.slice(target.lastIndexOf('--') + 2)))
				)
					continue;
				const meant = closest(target, ids.keys());
				report(
					element,
					'error',
					'missing-id',
					`${attribute}="${target}" matches no element id.${meant ? ` Did you mean "${meant}"?` : ''}`,
					attribute,
				);
			}
		}

		if (name === 'ex-explorer') {
			const file = attributes.get('src');
			const script = element.childNodes
				.filter(isElement)
				.find(
					(child): boolean =>
						child.tagName === 'script' && attributeMap(child).get('type') === 'application/json',
				);
			// oxlint-disable-next-line no-await-in-loop -- external content is checked in page order, once per explorer.
			const external = file && options.readSource ? await options.readSource(file) : undefined;
			const content = external ?? (script ? textOf(script) : undefined);
			if (content === undefined)
				report(
					element,
					'error',
					'explorer',
					file
						? `Cannot read explorer source "${file}"; provide the file next to the page.`
						: 'ex-explorer needs src or an application/json block.',
				);
			else {
				try {
					const graph = parseGraphDocument(JSON.parse(content));
					const selected = attributes.get('selected');
					if (selected && !graph.nodes.some((node): boolean => node.id === selected))
						report(
							element,
							'error',
							'explorer',
							`Unknown initial graph selection: ${selected}.`,
							'selected',
						);
				} catch (error) {
					report(
						element,
						'error',
						'explorer',
						error instanceof Error ? error.message : 'Invalid explorer data.',
					);
				}
			}
		}

		if (name === 'ex-tabs')
			for (const child of element.childNodes.filter(isElement))
				if (child.tagName !== 'section' || !attributeMap(child).has('label'))
					report(child, 'warning', 'tabs', 'Each view in ex-tabs is a <section label="…">.');

		if (name === 'ex-markdown' && attributes.has('src')) {
			const reference = attributes.get('src') ?? '';
			// oxlint-disable-next-line no-await-in-loop -- check each referenced source with the caller's resolver.
			const text = options.readSource ? await options.readSource(reference) : undefined;
			if (!reference.trim() || (options.readSource && text === undefined))
				report(
					element,
					'error',
					'markdown-source',
					`Cannot read Markdown source "${reference}"; provide the file beside the page or embed it.`,
					'src',
				);
		}

		// Island sources: markup inside the text means the browser already broke it into elements.
		if (spec.source) {
			const children = element.childNodes.filter(isElement);
			const script = children.find((child): boolean => child.tagName === 'script');
			const stray = children.filter(
				(child): boolean =>
					child.tagName !== 'script' && child.tagName !== 'template' && child.tagName !== 'ex-note',
			);
			if (!script && stray.length > 0)
				report(
					element,
					'error',
					'source',
					`<${name}> text was read as HTML (<${stray[0]?.tagName ?? ''}>); put it in <script type="${spec.source}">…</script>.`,
				);
			const type = script ? attributeMap(script).get('type') : undefined;
			if (script && attributeMap(script).has('data-ex-markdown-source') && name === 'ex-markdown') {
				try {
					parseMarkdownSource(textOf(script));
				} catch {
					report(script, 'error', 'markdown-source', 'Invalid embedded Markdown source.');
				}
			} else if (script && type !== spec.source)
				report(script, 'warning', 'source', `Use <script type="${spec.source}"> inside <${name}>.`);
		}

		// Notes must name a row of their code (and, in a multi-file diff, a file).
		if (name === 'ex-code') {
			const children = element.childNodes.filter(isElement);
			const carrier = children.find(
				(child): boolean => child.tagName === 'script' || child.tagName === 'template',
			);
			const lines = normalizeIslandText(carrier ? textOf(carrier) : '').split('\n');
			const diff =
				attributes.get('lang') === 'diff' || lines.some((line): boolean => line.startsWith('@@'));
			const files = parseCode(
				lines,
				diff,
				Number.parseInt(attributes.get('start') ?? '1', 10) || 1,
			);
			for (const note of children.filter((child): boolean => child.tagName === 'ex-note')) {
				const noteAttributes = attributeMap(note);
				const line = Number.parseInt(noteAttributes.get('line') ?? '', 10);
				const named = noteAttributes.get('file');
				if (files.length > 1 && !named) {
					report(
						note,
						'error',
						'note',
						`This diff has ${files.length} files; add file="…" to say which one line="${line}" is in.`,
					);
					continue;
				}
				const inFiles = files.filter(
					(file): boolean => !named || file.path?.endsWith(named) === true,
				);
				if (named && inFiles.length === 0) {
					report(
						note,
						'error',
						'note',
						`file="${named}" is not a file in this diff (${files.map((file) => file.path ?? '?').join(', ')}).`,
						'file',
					);
					continue;
				}
				const numbers = inFiles.flatMap((file): number[] =>
					file.lines.flatMap((codeLine): number[] =>
						codeLine.number === undefined ? [] : [codeLine.number],
					),
				);
				if (!numbers.includes(line))
					report(
						note,
						'error',
						'note',
						`line="${noteAttributes.get('line') ?? ''}" is not a row of this code; rows are ${Math.min(...numbers)}–${Math.max(...numbers)}${diff ? ' (every row counts, removed lines included; a @@ header restarts at its + number)' : ''}.`,
						'line',
					);
			}
		}

		if (name === 'ex-mermaid' && options.diagrams !== false) {
			const script = element.childNodes
				.filter(isElement)
				.find((child): boolean => child.tagName === 'script');
			const file = attributes.get('src');
			// oxlint-disable-next-line no-await-in-loop -- diagram files are read one at a time, in page order.
			const external = file && options.readSource ? await options.readSource(file) : undefined;
			if (file && external === undefined) {
				report(
					element,
					'warning',
					'diagram',
					`src="${file}" is not a file next to the page; the diagram renders only if a packager supplies it.`,
					'src',
				);
				continue;
			}
			const diagram = normalizeIslandText(external ?? (script ? textOf(script) : textOf(element)));
			try {
				if (!engine) continue;
				const svg = renderThemedDiagram(engine.renderMermaidSVG, diagram);
				for (const match of svg.matchAll(
					/<g class="subgraph" data-id="([^"]*)"[^>]*>\s*<rect[^>]*width="0"/g,
				))
					report(
						element,
						'error',
						'diagram',
						`Subgraph "${match[1] ?? ''}" is empty: declare its nodes inside the subgraph block.`,
					);
				if (hasGraphEdges(diagram) && !/class="edge"/.test(svg))
					report(
						element,
						'error',
						'diagram',
						'No arrows rendered: check the arrow syntax, e.g. a --> b.',
					);
			} catch (error) {
				report(
					element,
					'error',
					'diagram',
					`Diagram does not render: ${String(error).replace(/^Error: /, '')}`,
				);
			}
		}
	}

	// Bare Markdown that an HTML formatter re-wrapped: a heading joined to its paragraph, or a list
	// joined onto one line. Both still parse, so only the source shows the damage.
	for (const element of elements) {
		if (!markdownHosts.has(element.tagName)) continue;
		for (const child of element.childNodes) {
			if (!('value' in child) || typeof child.value !== 'string' || isElement(child)) continue;
			const firstLine = child.sourceCodeLocation?.startLine ?? 1;
			const lines = child.value.split('\n');
			lines.forEach((line, index): void => {
				const place = { line: firstLine + index, column: line.search(/\S/) + 1 };
				const heading = /^\s*#{1,6}\s+(.+)$/.exec(line);
				const next = lines[index + 1]?.trim() ?? '';
				if (heading && next && !next.startsWith('<'))
					findings.push({
						...place,
						severity: 'warning',
						rule: 'markdown',
						message:
							'Put a blank line after each heading; a formatter has joined this heading to the text below it.',
					});
				if (/^\s*[-*+]\s/.test(line) && (line.match(/\s-\s/g) ?? []).length >= 2)
					findings.push({
						...place,
						severity: 'warning',
						rule: 'markdown',
						message:
							'Several list items share one line; a formatter has joined the list. Put each "- item" on its own line.',
					});
			});
		}
	}

	if (!loadsTheme || !loadsRuntime) {
		const head = elements.find((element): boolean => element.tagName === 'head');
		const missing = [
			loadsTheme ? '' : '<link rel="stylesheet" href="…/theme.css">',
			loadsRuntime ? '' : '<script type="module" src="…/runtime.js"></script>',
		].filter(Boolean);
		if (head)
			report(head, 'error', 'library', `Load the library in <head>: ${missing.join(' and ')}.`);
	}
	return findings.toSorted(
		(first, second): number => first.line - second.line || first.column - second.column,
	);
}

/** Applies every finding's edit, last first so earlier offsets stay valid. */
export function applyEdits(source: string, findings: readonly Finding[]): string {
	const edits = findings
		.flatMap((finding): TextEdit[] => (finding.edit ? [finding.edit] : []))
		.toSorted((first, second): number => second.start - first.start);
	let result = source;
	let limit = Infinity;
	for (const edit of edits) {
		if (edit.end > limit) continue;
		result = result.slice(0, edit.start) + edit.text + result.slice(edit.end);
		limit = edit.start;
	}
	return result;
}

/**
 * One line per problem, the form an agent reads fastest. A problem repeated on several lines is
 * printed once with every line number, so a page with twenty untoned cards costs one line.
 */
export function formatFindings(file: string, findings: readonly Finding[]): string {
	const groups = new Map<string, Finding[]>();
	for (const finding of findings) {
		const key = `${finding.severity} ${finding.rule}: ${finding.message}${finding.edit ? ' (fixable)' : ''}`;
		groups.set(key, [...(groups.get(key) ?? []), finding]);
	}
	return Array.from(groups, ([key, group]): string => {
		const [first] = group;
		const place =
			group.length === 1 && first
				? `${first.line}:${first.column}`
				: group.map((finding): number => finding.line).join(',');
		return `${file}:${place} ${key}`;
	}).join('\n');
}
