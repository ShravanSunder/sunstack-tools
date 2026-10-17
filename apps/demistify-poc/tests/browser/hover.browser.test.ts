import { afterEach, expect, test, vi } from 'vitest';

import {
	compositionView,
	type GraphDocument,
} from '../../../../packages/demistify/src/explorer/graph-document.ts';
import { renderExplorerGraph } from '../../../../packages/demistify/src/explorer/graph-renderer.ts';
import {
	installArrowTracing,
	setTrace,
	traceDiagram,
} from '../../../../packages/demistify/src/layout/trace.ts';

interface HoverSurface {
	readonly owner: Element;
	readonly first: Element;
	readonly second: Element;
	readonly traced: () => readonly string[];
}

const graph: GraphDocument = {
	version: 1,
	sourceBase: 'https://example.com/',
	title: 'Hover fixture',
	description: '',
	note: '',
	groups: [{ id: 'owners', label: 'Owners', kind: 'method' }],
	nodes: ['alpha', 'beta', 'gamma'].map((id) => ({
		id,
		label: id,
		description: '',
		group: 'owners',
		issue: '',
		status: 'current',
		sources: [],
	})),
	edges: [
		{
			id: 'alpha-beta',
			source: 'alpha',
			target: 'beta',
			expansion: 'composition',
			status: 'current',
			label: 'uses',
			why: '',
			when: '',
			use: '',
			sources: [],
		},
		{
			id: 'beta-gamma',
			source: 'beta',
			target: 'gamma',
			expansion: 'composition',
			status: 'current',
			label: 'uses',
			why: '',
			when: '',
			use: '',
			sources: [],
		},
	],
	tables: [],
};

function required(root: ParentNode, selector: string): Element {
	const element = root.querySelector(selector);
	if (!element) throw new Error(`Missing hover fixture: ${selector}`);
	return element;
}

let arrowsInstalled = false;

function createSurface(kind: string): HoverSurface {
	const container = document.createElement('div');
	document.body.append(container);
	if (kind === 'explorer') {
		renderExplorerGraph({
			viewport: container,
			document: graph,
			view: compositionView(graph, 'alpha', 'current'),
			onSelect: (): void => {},
			onConnection: (): void => {},
		});
		return {
			owner: container.firstElementChild ?? container,
			first: required(container, '[data-node-id="beta"]'),
			second: required(container, '[data-node-id="alpha"]'),
			traced: (): readonly string[] =>
				[...container.querySelectorAll('.is-traced')]
					.map((edge): string => edge.getAttribute('data-edge-id') ?? '')
					.toSorted(),
		};
	}
	container.innerHTML = `<svg class="${kind === 'arrows' ? 'ex-arrow-canvas' : kind === 'flow' ? 'ex-flow-overlay' : ''}"><g data-id="alpha" tabindex="0"><rect width="80" height="40"></rect></g><g data-id="beta" tabindex="0"><rect width="80" height="40"></rect></g><g data-id="gamma" tabindex="0"><rect width="80" height="40"></rect></g><path data-from="alpha" data-to="beta" data-edge-id="alpha-beta"></path><path data-from="beta" data-to="gamma" data-edge-id="beta-gamma"></path></svg>`;
	const svg = required(container, 'svg');
	if (kind === 'mermaid') {
		if (!(svg instanceof SVGSVGElement)) throw new Error('Expected SVG fixture.');
		traceDiagram(svg);
	} else {
		container.insertAdjacentHTML(
			'beforeend',
			'<button id="alpha">Alpha</button><button id="beta">Beta</button><button id="gamma">Gamma</button><ex-arrow from="alpha" to="beta"></ex-arrow><ex-arrow from="beta" to="gamma"></ex-arrow>',
		);
		if (!arrowsInstalled) {
			installArrowTracing();
			arrowsInstalled = true;
		}
	}
	return {
		owner: svg,
		first: required(container, kind === 'mermaid' ? '[data-id="beta"]' : '#beta'),
		second: required(container, kind === 'mermaid' ? '[data-id="alpha"]' : '#alpha'),
		traced: (): readonly string[] =>
			[...svg.querySelectorAll('[data-edge-id].ex-traced')]
				.map((edge): string => edge.getAttribute('data-edge-id') ?? '')
				.toSorted(),
	};
}

function point(element: Element, kind: string, entering: boolean): void {
	element.dispatchEvent(
		new PointerEvent(
			kind === 'arrows' || kind === 'flow'
				? 'pointerover'
				: entering
					? 'pointerenter'
					: 'pointerleave',
			{ bubbles: true, pointerType: 'mouse' },
		),
	);
}

afterEach((): void => {
	document.body.dispatchEvent(new PointerEvent('pointerover', { bubbles: true }));
	vi.runOnlyPendingTimers();
	document.body.replaceChildren();
	vi.useRealTimers();
});

test.each(['mermaid', 'arrows', 'flow', 'explorer'])(
	'%s hover waits 100ms on entry and exit, and rapid motion settles only its latest target',
	(kind): void => {
		vi.useFakeTimers();
		const surface = createSurface(kind);
		point(surface.first, kind, true);
		expect(surface.traced()).toEqual([]);
		vi.advanceTimersByTime(99);
		expect(surface.traced()).toEqual([]);
		vi.advanceTimersByTime(1);
		expect(surface.traced()).toEqual(['alpha-beta', 'beta-gamma']);
		if (kind === 'arrows' || kind === 'flow')
			document.documentElement.dispatchEvent(new PointerEvent('pointerleave'));
		else point(surface.first, kind, false);
		vi.advanceTimersByTime(99);
		expect(surface.traced()).toEqual(['alpha-beta', 'beta-gamma']);
		vi.advanceTimersByTime(1);
		expect(surface.traced()).toEqual([]);
		point(surface.first, kind, true);
		vi.advanceTimersByTime(60);
		point(surface.second, kind, true);
		vi.advanceTimersByTime(99);
		expect(surface.traced()).toEqual([]);
		vi.advanceTimersByTime(1);
		expect(surface.traced()).toEqual(['alpha-beta']);
	},
);

test.each(['mermaid', 'arrows', 'flow', 'explorer'])(
	'%s focus remains immediate and cancels an older hover; detached pictures receive no late paint',
	(kind): void => {
		vi.useFakeTimers();
		const surface = createSurface(kind);
		point(surface.first, kind, true);
		if (!(surface.second instanceof HTMLElement || surface.second instanceof SVGElement))
			throw new Error('Expected a focusable node.');
		surface.second.focus();
		expect(surface.traced()).toEqual(['alpha-beta']);
		vi.advanceTimersByTime(100);
		expect(surface.traced()).toEqual(['alpha-beta']);
		point(surface.first, kind, true);
		surface.owner.remove();
		const detachedClasses = (): readonly (string | null)[] =>
			[...surface.owner.querySelectorAll('[data-edge-id]')].map((edge) =>
				edge.getAttribute('class'),
			);
		const before = detachedClasses();
		vi.advanceTimersByTime(100);
		expect(detachedClasses()).toEqual(before);
	},
);

test('presentation trace cancels a pending diagram hover immediately', (): void => {
	vi.useFakeTimers();
	const surface = createSurface('mermaid');
	point(surface.first, 'mermaid', true);
	setTrace(surface.owner, 'alpha');
	expect(surface.traced()).toEqual(['alpha-beta']);
	vi.advanceTimersByTime(100);
	expect(surface.traced()).toEqual(['alpha-beta']);
});

test.each(['mermaid', 'arrows', 'flow', 'explorer'])(
	'%s actions resolve a pending hover exit instead of leaving the old trace stuck',
	(kind): void => {
		vi.useFakeTimers();
		const surface = createSurface(kind);
		point(surface.first, kind, true);
		vi.advanceTimersByTime(100);
		expect(surface.traced()).toEqual(['alpha-beta', 'beta-gamma']);
		if (kind === 'arrows' || kind === 'flow')
			document.body.dispatchEvent(new PointerEvent('pointerover', { bubbles: true }));
		else point(surface.first, kind, false);
		surface.owner.dispatchEvent(new MouseEvent('click', { bubbles: true }));
		expect(surface.traced()).toEqual([]);
		vi.advanceTimersByTime(100);
		expect(surface.traced()).toEqual([]);
		point(surface.first, kind, true);
		vi.advanceTimersByTime(100);
		if (kind === 'arrows' || kind === 'flow')
			document.body.dispatchEvent(new PointerEvent('pointerover', { bubbles: true }));
		else point(surface.first, kind, false);
		surface.owner.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
		expect(surface.traced()).toEqual([]);
	},
);

test.each(['arrows', 'flow'])(
	'%s presentation events discard a pending hover without leaving its old trace',
	(kind): void => {
		vi.useFakeTimers();
		const surface = createSurface(kind);
		point(surface.first, kind, true);
		vi.advanceTimersByTime(100);
		expect(surface.traced()).toEqual(['alpha-beta', 'beta-gamma']);
		document.body.dispatchEvent(new PointerEvent('pointerover', { bubbles: true }));
		document.dispatchEvent(new Event('presentation-target'));
		expect(surface.traced()).toEqual([]);
		point(surface.first, kind, true);
		document.dispatchEvent(new Event('presentation-change'));
		vi.advanceTimersByTime(100);
		expect(surface.traced()).toEqual([]);
		point(surface.first, kind, true);
		vi.advanceTimersByTime(100);
		expect(surface.traced()).toEqual(['alpha-beta', 'beta-gamma']);
	},
);
