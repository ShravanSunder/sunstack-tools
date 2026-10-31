/**
 * The authoring vocabulary: every element, the attributes it accepts and what they mean. It is the
 * one source for the runtime's attribute checks, the `demistify check` CLI, the element manifest
 * and editor completions, so the contract cannot drift between them. Pure data; safe in Node.
 */
import { subjectIconAliases, subjectIcons } from './core/subject-icons.ts';
import { toneAliases, type ToneName, toneNames } from './core/tone-names.ts';

export { type ToneName, toneNames };

export type AttributeType =
	| { readonly kind: 'text' }
	| { readonly kind: 'flag' }
	| { readonly kind: 'enum'; readonly values: readonly string[] }
	| { readonly kind: 'number'; readonly min: number; readonly max: number }
	| { readonly kind: 'id' }
	| { readonly kind: 'ids' };

export interface AttributeSpec {
	readonly description: string;
	readonly type: AttributeType;
	readonly required?: boolean;
	/** Missing it is allowed but worth a warning (e.g. a card without a tone has no icon). */
	readonly recommended?: boolean;
}

export interface ElementSpec {
	readonly description: string;
	readonly attributes: Readonly<Record<string, AttributeSpec>>;
	readonly childTag?: string;
	/** Attributes this element gives its direct children (e.g. `span` inside a grid). */
	readonly childAttributes?: Readonly<Record<string, AttributeSpec>>;
	/** The element must sit inside one of these. */
	readonly within?: readonly string[];
	/** Text the element reads from a `<script type="…">` child. */
	readonly source?: 'text/plain' | 'text/markdown';
	/** Earlier lab vocabulary: still rendered, not taught. */
	readonly legacy?: boolean;
}

const text = (description: string, required = false): AttributeSpec => ({
	description,
	type: { kind: 'text' },
	required,
});
const flag = (description: string): AttributeSpec => ({ description, type: { kind: 'flag' } });
const oneOf = (description: string, values: readonly string[]): AttributeSpec => ({
	description,
	type: { kind: 'enum', values },
});
const tone: AttributeSpec = oneOf('What kind of thing this is; sets icon and colour.', toneNames);
const toneExpected: AttributeSpec = { ...tone, recommended: true };
const icon = oneOf('What the thing is (a curated icon); the tone still colours it.', subjectIcons);
const label = text('Name used by the inspector, tabs and routes.');
const categorySeries = oneOf(
	'Explicit category palette slot, separate from tone/status; omit for neutral.',
	['1', '2', '3', '4', '5', '6'],
);
const columns: AttributeSpec = {
	description: 'Most columns side by side (1–6); fewer when narrow, one on phones.',
	type: { kind: 'number', min: 1, max: 6 },
};
const sides = ['left', 'right', 'top', 'bottom'];
const gridChildren: Readonly<Record<string, AttributeSpec>> = {
	span: oneOf('Columns this child spans while the grid is wide.', ['2', '3', 'all']),
	sticky: flag('Keeps this column in view while the rest scrolls.'),
};

export const layoutAttributes: Readonly<Record<string, AttributeSpec>> = {
	'data-width': oneOf(
		'Width of this entire component or layout wrapper; Reading and Canvas share a left edge.',
		['reading', 'canvas'],
	),
};

export const vocabulary: Readonly<Record<string, ElementSpec>> = {
	'ex-choice': {
		description: 'One question with native single or multiple response options.',
		attributes: {
			id: text('Stable authored question identity.', true),
			title: text('The question readers answer.', true),
			multiple: flag('Allow independent selected options instead of one answer.'),
			view: oneOf('Cards at Reading width, or a selectable table at Canvas width.', [
				'cards',
				'table',
			]),
			details: oneOf('Extra content opens in the native sidebar, or an inline disclosure.', [
				'sidebar',
				'inline',
			]),
		},
		childTag: 'ex-option',
	},
	'ex-option': {
		description:
			'An identified response option with inline title, summary template and optional detail slot.',
		attributes: {
			id: text('Stable authored option identity.', true),
			title: text('One paragraph of inline Markdown.', true),
		},
		within: ['ex-choice'],
	},
	'ex-comments': {
		description:
			'One local review with comments and current answers; supplied automatically for ordinary documents.',
		attributes: {
			for: {
				description: 'Original ex-document DOM ID. Missing logical identity/revision are derived.',
				type: { kind: 'id' },
				required: true,
			},
		},
	},
	'ex-pre': {
		description: 'Literal preformatted text and TUI drawings with shared inspection.',
		attributes: { label },
		source: 'text/plain',
	},
	'ex-gallery': {
		description:
			'Dedicated Gallery page with agent-authored figures, shared review/grid views, filters and fullscreen inspection.',
		attributes: {
			label,
			filters: text('Space-separated metadata fields for filters.'),
			view: oneOf('Initial gallery layout.', ['review', 'grid']),
			selected: text('Initial authored capture id.'),
		},
	},
	'ex-explorer': {
		description:
			'Searchable graph document with shared selection, history, tracing, evidence and related tables.',
		attributes: {
			src: text('Relative graph document JSON; alternatively embed an application/json script.'),
			selected: text('Initial graph node id.'),
			view: oneOf('Initial relationship view.', ['current', 'proposed']),
			label,
		},
	},
	'ex-document': {
		description:
			'The page body: bare Markdown, islands and layouts; adds outline and theme toggle.',
		attributes: {
			'document-id': text(
				'Logical review identity override; defaults to the page URL without query/fragment.',
			),
			revision: text('Content revision override; defaults to an authored-content fingerprint.'),
		},
	},
	'ex-presentation': {
		description:
			'A named/described ordered path through existing targets; content remains in its document.',
		attributes: {
			title: text('Presentation name shown in the menu.', true),
			variations: flag('Allow same-screen viewport variations with buttons and Up/Down keys.'),
			description: text('Short description shown beneath its name.', true),
		},
	},
	'ex-markdown': {
		description:
			'Markdown inline or from a file; source stays inspectable beside the rendered content.',
		attributes: {
			label,
			src: text('Markdown URL relative to this page; embed local files for file-protocol viewing.'),
		},
		source: 'text/markdown',
	},
	'ex-card': {
		description: 'One thing (component, entity, step, option, finding); body in Markdown.',
		attributes: {
			title: text('The specific thing, shown as the card heading.'),
			tone: toneExpected,
			series: categorySeries,
			icon,
			tag: text('Short tag beside the title, e.g. a tradeoff or location.'),
			pick: flag('Marks the chosen option.'),
			label,
		},
	},
	'ex-callout': {
		description: 'TL;DR, decision, risk, gotchas or open questions.',
		attributes: {
			title: text('Optional heading.'),
			tone,
			icon,
			type: text('Legacy name for tone.'),
			label,
		},
	},
	'ex-stat': {
		description: 'One number with its label and an optional comparison note.',
		attributes: {
			value: text('The number, with its unit.', true),
			label: text('What the number measures.', true),
			note: text('Comparison or context, e.g. "was 443 KiB".'),
			tone,
		},
	},
	'ex-badge': {
		description: 'A normal-text label with an optional existing semantic tone.',
		attributes: { tone },
	},
	'ex-progress': {
		description: 'One exact completed count and total; missing counts remain not reported.',
		attributes: {
			label: text('Subject named beside the count.', true),
			value: text('Authored completed whole-number count.'),
			max: text('Authored whole-number total for the same subject.'),
		},
	},
	'ex-breakdown': {
		description: 'One named observation with one to three exact, stably slotted phase values.',
		attributes: {
			label: text('Name of this observation.', true),
			unit: text('Shared unit printed with each phase value.', true),
			scale: oneOf('Share of the sum or elapsed length against max.', ['share', 'elapsed']),
			max: text('Required positive common maximum for elapsed scale.'),
			compact: flag('Keep exact phase values in order for a labelled surrounding table.'),
		},
		childTag: 'data',
		childAttributes: {
			value: text('Finite non-negative exact phase value.', true),
			series: oneOf('Stable categorical palette slot for this phase.', ['1', '2', '3']),
		},
	},
	'ex-mermaid': {
		description:
			'A Mermaid diagram or chart with shared categorical colors; flowchart classes slot1–slot6 pin authored categories. Nodes whose id matches a card open that card.',
		attributes: { label, src: text('Diagram file, inlined when the page is packaged.') },
		source: 'text/plain',
	},
	'ex-code': {
		description: 'Code or a diff with margin notes.',
		attributes: {
			title: text('File name or what the code is.'),
			lang: text('Language; "diff" for a unified diff.'),
			start: {
				description: 'Number of the first line.',
				type: { kind: 'number', min: 1, max: 1_000_000 },
			},
			label,
		},
		source: 'text/plain',
	},
	'ex-note': {
		description: 'A margin note on one line of the enclosing ex-code.',
		attributes: {
			line: {
				description: 'Line number the note belongs to.',
				type: { kind: 'number', min: 1, max: 1_000_000 },
				required: true,
			},
			file: text('In a multi-file diff, the file the line is in (path or its end).'),
			tone,
		},
		within: ['ex-code'],
	},
	'ex-file-tree': {
		description: 'A plain-text file tree.',
		attributes: { label },
		source: 'text/plain',
	},
	'ex-tree': {
		description:
			'A Markdown list as a tree: "tone: text" gives a node its icon, [x]/[ ] make a checklist, [text](#id) jumps to an island.',
		attributes: {
			title: text('Root node.'),
			tone,
			icon,
			layout: oneOf('outline (default) or a left-to-right mind map.', ['outline', 'map']),
			label,
		},
	},
	'ex-trace': {
		description:
			'Ordered native disclosures with rich evidence attachments, distinct from a hierarchy tree.',
		attributes: { title: text('Name of this evidence sequence.'), tone, icon, label },
		childAttributes: { tone, icon, tag: text('Short source or observation status on a step.') },
	},
	'ex-grid': {
		description: 'Children side by side; wraps when narrow, one column on phones.',
		attributes: { cols: columns, swipe: flag('A sideways swipe row on phones.') },
		childAttributes: gridChildren,
	},
	'ex-flow': {
		description: 'A grid whose ex-arrow children route through the gaps between boxes.',
		attributes: { cols: columns },
		childAttributes: gridChildren,
	},
	'ex-arrow': {
		description: 'A labelled arrow between two elements; routes around every other box.',
		attributes: {
			from: { description: 'Id of the start element.', type: { kind: 'id' }, required: true },
			to: { description: 'Id of the end element.', type: { kind: 'id' }, required: true },
			label: text('Words on the arrow.'),
			'from-anchor': oneOf('Side the arrow leaves from.', sides),
			'to-anchor': oneOf('Side the arrow arrives at.', sides),
		},
	},
	'ex-tabs': {
		description: 'Views as tabs; each child <section label> is one view.',
		attributes: { columns: flag('Shows the views side by side when they fit.'), label },
	},
	'ex-board': {
		description:
			'A pannable, zoomable map: of ex-region groups, or of your own layout (absolute, grid or flex) when it has no regions.',
		attributes: {
			cols: columns,
			mode: oneOf(
				'read (default) shows the board like the page; map starts as a pannable, zoomable map, a deliberate choice for an overview.',
				['map', 'read'],
			),
		},
	},
	'ex-region': {
		description: 'A titled group on a board; children marked `detail` show when it opens.',
		attributes: {
			title: text('Region heading.'),
			tone: toneExpected,
			series: categorySeries,
			icon,
			label,
		},
		childAttributes: { detail: flag('Hidden on the map until the region is opened.') },
		within: ['ex-board'],
	},
	'ex-step': {
		description: 'One stop on a route; its text is focused Markdown narration.',
		attributes: {
			to: {
				description: 'Id of the island, region or diagram node; or another page, page.html#id.',
				type: { kind: 'id' },
				required: true,
			},
			open: oneOf('Open the target in the side panel or expanded.', ['sidebar', 'modal']),
			icon,
		},
		source: 'text/markdown',
		within: ['ex-presentation'],
	},
	'ex-layers': {
		description:
			'The layer switches; added at the top automatically when any element has layer="…".',
		attributes: {
			off: text('Layers that start hidden, space-separated.'),
			label: text('Text before the switches (default "Show").'),
		},
	},
	'ex-nav': {
		description: 'Heading navigation; added automatically by ex-document.',
		attributes: { for: { description: 'Id of the content to outline.', type: { kind: 'id' } } },
	},
	'ex-zoom': {
		description: 'A surface whose `slot="details"` children show only when inspected.',
		attributes: { label },
		legacy: true,
	},
	'ex-decision': {
		description: 'A decision record.',
		attributes: { status: text('Decision status.'), label },
		legacy: true,
	},
	'ex-compare': {
		description:
			'Alternatives with arbitrary islands: corresponding topic rows align, or one detailed view appears in tabs. A slider overlays two same-size pictures.',
		attributes: {
			slider: flag('Overlay two image sides with a divider.'),
			view: oneOf('Authored content presentation; readers do not configure layout.', [
				'columns',
				'tabs',
			]),
			choose: flag('Let readers choose a pair of alternatives.'),
			selected: {
				description: 'Initial child alternative IDs, in display order.',
				type: { kind: 'ids' },
			},
			label,
		},
		childAttributes: { label: text('Name of this alternative (default Before / After).'), icon },
	},
	'ex-diagram-diff': {
		description: 'Before and after diagrams.',
		attributes: { label },
		legacy: true,
	},
	'ex-timeline': { description: 'A vertical timeline.', attributes: { label }, legacy: true },
	'ex-layer': { description: 'A layer band.', attributes: { label }, legacy: true },
};

/** Attributes valid on every element. */
const globalAttributes = new Set([
	'id',
	'class',
	'style',
	'slot',
	'hidden',
	'lang',
	'dir',
	'tabindex',
	'role',
	'title',
	'popover',
]);
const islandAttributes: Readonly<Record<string, AttributeSpec>> = {
	layer: text('A layer readers can switch on and off (e.g. "proposed").'),
	zoom: oneOf('Bare `zoom` adds sidebar and expand buttons; "none" removes them.', ['', 'none']),
};

/** Common mistakes mapped to the attribute or value that was meant. */
const attributeAliases: Readonly<Record<string, string>> = {
	type: 'tone',
	kind: 'tone',
	variant: 'tone',
	color: 'tone',
	severity: 'tone',
	status: 'tone',
	heading: 'title',
	name: 'title',
	columns: 'cols',
	target: 'to',
	href: 'to',
	language: 'lang',
	'start-line': 'start',
};

export interface AttributeProblem {
	readonly attribute: string;
	readonly message: string;
	readonly severity?: 'warning';
	/** A safe mechanical repair: rename the attribute and/or replace its value. */
	readonly fix?: { readonly name?: string; readonly value?: string };
}

function distance(first: string, second: string): number {
	const row = Array.from({ length: second.length + 1 }, (_, index): number => index);
	for (let outer = 1; outer <= first.length; outer += 1) {
		let diagonal = row[0] ?? 0;
		row[0] = outer;
		for (let inner = 1; inner <= second.length; inner += 1) {
			const above = row[inner] ?? 0;
			row[inner] = Math.min(
				above + 1,
				(row[inner - 1] ?? 0) + 1,
				diagonal + (first[outer - 1] === second[inner - 1] ? 0 : 1),
			);
			diagonal = above;
		}
	}
	return row[second.length] ?? 0;
}

/** The closest known name within two edits, if exactly one is that close. */
export function closest(name: string, candidates: Iterable<string>): string | undefined {
	const near = Array.from(candidates).filter(
		(candidate): boolean => distance(name, candidate) <= 2,
	);
	return near.length === 1 ? near[0] : undefined;
}

function checkValue(
	name: string,
	value: string,
	spec: AttributeSpec,
): AttributeProblem | undefined {
	const { type } = spec;
	if (type.kind === 'flag' && (value === 'false' || value === 'no'))
		return {
			attribute: name,
			severity: 'warning',
			message: `${name}="${value}" still turns ${name} on; remove the attribute instead.`,
		};
	if (type.kind === 'enum' && !type.values.includes(value)) {
		const aliases: Readonly<Record<string, string>> | undefined =
			type.values === toneNames
				? toneAliases
				: type.values === subjectIcons
					? subjectIconAliases
					: undefined;
		const alias = aliases?.[value];
		const meant = alias ?? closest(value, type.values);
		return {
			attribute: name,
			// A known synonym still works at runtime; it is only worth writing the real name.
			...(alias ? { severity: 'warning' as const } : {}),
			message: meant
				? `${name}="${value}" is not a known ${name}; write ${name}="${meant}".`
				: `${name}="${value}" is not one of: ${type.values.join(', ')}.`,
			...(meant ? { fix: { value: meant } } : {}),
		};
	}
	if (type.kind === 'number') {
		const number = Number(value);
		if (!Number.isInteger(number) || number < type.min || number > type.max)
			return {
				attribute: name,
				message: `${name}="${value}" must be a whole number from ${type.min} to ${type.max}.`,
			};
	}
	if ((type.kind === 'id' || type.kind === 'ids' || spec.required) && value.trim() === '')
		return { attribute: name, message: `${name} is empty.` };
	return undefined;
}

/**
 * Checks one element's attributes against the vocabulary. `parent` is the local name of its parent
 * element, which may grant child attributes such as `span` or `detail`.
 */
export function checkAttributes(
	element: string,
	attributes: ReadonlyMap<string, string>,
	parent: string | undefined,
): AttributeProblem[] {
	const spec = vocabulary[element];
	const granted = parent ? vocabulary[parent]?.childAttributes : undefined;
	const problems: AttributeProblem[] = [];
	if (spec)
		for (const [name, attribute] of Object.entries(spec.attributes))
			if (attribute.required && !attributes.has(name))
				problems.push({ attribute: name, message: `<${element}> needs ${name}="…".` });
			else if (attribute.recommended && !attributes.has(name))
				problems.push({
					attribute: name,
					severity: 'warning',
					message:
						name === 'tone'
							? `<${element}> has no tone, so no ${attributes.has('icon') ? '' : 'icon or '}colour; add tone="…" for its status or kind (timing and location go in tag).`
							: `<${element}> should have ${name}="…".`,
				});
	for (const [name, value] of attributes) {
		if (
			globalAttributes.has(name) ||
			(name.startsWith('data-') && !layoutAttributes[name]) ||
			name.startsWith('aria-')
		)
			continue;
		const known =
			layoutAttributes[name] ??
			granted?.[name] ??
			spec?.attributes[name] ??
			(spec ? islandAttributes[name] : undefined);
		if (known) {
			// The legacy free-form callout type stays valid; only a tone written as type is moved.
			if (element === 'ex-callout' && name === 'type') {
				if ((toneNames as readonly string[]).includes(value))
					problems.push({
						attribute: name,
						severity: 'warning',
						message: `Write tone="${value}" instead of type.`,
						fix: { name: 'tone' },
					});
				continue;
			}
			const problem = checkValue(name, value, known);
			if (problem) problems.push(problem);
			continue;
		}
		if (!spec) continue;
		const names = [...Object.keys(spec?.attributes ?? {}), ...Object.keys(granted ?? {})];
		const alias = attributeAliases[name];
		const meant = alias && names.includes(alias) ? alias : closest(name, names);
		// Moving a value into tone also maps tone synonyms (type="warning" becomes tone="risk").
		const movedValue =
			meant === 'tone' && !(toneNames as readonly string[]).includes(value)
				? (toneAliases[value] ?? closest(value, toneNames))
				: undefined;
		const host =
			granted?.[name] === undefined && parent && !spec ? `<${parent}> child` : `<${element}>`;
		problems.push({
			attribute: name,
			...(spec?.legacy ? { severity: 'warning' as const } : {}),
			message: `${host} has no attribute "${name}"${spec?.legacy ? '; it has no effect' : ''}.${meant ? ` Use ${meant}.` : ` Known: ${names.join(', ') || 'none'}.`}`,
			...(meant && !attributes.has(meant)
				? { fix: movedValue ? { name: meant, value: movedValue } : { name: meant } }
				: {}),
		});
	}
	return problems;
}

/** Elements authors are taught, for suggestions; legacy elements are accepted but not suggested. */
export const taughtElements = Object.entries(vocabulary)
	.filter(([, spec]): boolean => !spec.legacy)
	.map(([name]): string => name);
