import {
	bindTabKeyboard,
	createModeChoices,
	createSearchField,
	labelControl,
} from '../core/controls.ts';
import { createDebouncedAction } from '../core/debounce.ts';
import { createIconButton } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import { registerInspectionCopy } from '../islands/inspection-copy.ts';
import { appendSources, connectionTable, supplementaryTable } from './explorer-tables.ts';
import {
	compositionView,
	parseGraphDocument,
	type CompositionView,
	type ExplorerMode,
	type ExplorerNode,
	type GraphDocument,
} from './graph-document.ts';
import { renderExplorerGraph } from './graph-renderer.ts';

/** One transient selection owns the graph, browser, breadcrumb, evidence and related tables. */
export class ExplorerController {
	private readonly graphDocument: GraphDocument;
	private readonly nodeById: ReadonlyMap<string, ExplorerNode>;
	private selection: string;
	private mode: ExplorerMode;
	private readonly history: string[] = [];
	private selectedEdge = '';
	private activeTab = 'connections';
	private view: CompositionView;
	private readonly browser = document.createElement('nav');
	private readonly searchField = createSearchField('Find a node', 'Find a node…');
	private readonly search = this.searchField.input;
	private readonly pendingSearch = createDebouncedAction((): void => this.renderBrowser(), 200);
	private readonly list = document.createElement('div');
	private readonly main = document.createElement('main');
	private readonly heading = document.createElement('section');
	private readonly breadcrumb = document.createElement('nav');
	private readonly overview = document.createElement('div');
	private readonly controls = document.createElement('div');
	private readonly viewport = document.createElement('div');
	private readonly related = document.createElement('details');
	private readonly tablist = document.createElement('div');
	private readonly panel = document.createElement('section');
	private readonly sources = document.createElement('aside');
	private readonly resizeObserver: ResizeObserver;
	private graphWidth = 0;

	constructor(
		private readonly owner: HTMLElement,
		graphDocument: GraphDocument,
	) {
		owner.id ||= `ex-explorer-${crypto.randomUUID()}`;
		this.graphDocument = graphDocument;
		registerInspectionCopy(owner, (): HTMLElement => {
			const copy = document.createElement('ex-explorer');
			copy.id = owner.id;
			copy.setAttribute('selected', this.selection);
			copy.setAttribute('view', this.mode);
			const source = document.createElement('script');
			source.type = 'application/json';
			source.textContent = JSON.stringify(this.graphDocument);
			copy.append(source);
			return copy;
		});
		this.nodeById = new Map(graphDocument.nodes.map((node) => [node.id, node]));
		const requested = owner.getAttribute('selected') ?? graphDocument.nodes[0]?.id ?? '';
		this.selection = this.nodeById.has(requested) ? requested : (graphDocument.nodes[0]?.id ?? '');
		const urlSelection = new URL(location.href).searchParams.get(`ex.${owner.id}`);
		if (urlSelection && (urlSelection === '__issues' || this.nodeById.has(urlSelection)))
			this.selection = urlSelection;
		this.mode = owner.getAttribute('view') === 'current' ? 'current' : 'proposed';
		this.view = compositionView(graphDocument, this.selection, this.mode);
		this.browser.className = 'ex-explorer-browser';
		this.browser.setAttribute('aria-label', 'Explore graph nodes');
		this.search.addEventListener('input', (): void => this.pendingSearch.schedule());
		const issues = document.createElement('button');
		issues.type = 'button';
		issues.className = 'ex-explorer-issues';
		labelControl(
			issues,
			'warning',
			`Composition issues · ${graphDocument.nodes.filter((node): boolean => Boolean(node.issue)).length}`,
		);
		issues.addEventListener('click', (): void => this.select('__issues'));
		const browserHeader = document.createElement('div');
		browserHeader.className = 'ex-explorer-browser-header';
		browserHeader.append(this.searchField.element, issues);
		this.list.className = 'ex-explorer-node-list';
		this.browser.append(browserHeader, this.list);
		const header = document.createElement('header');
		const title = document.createElement('h1');
		title.textContent = graphDocument.title;
		const description = document.createElement('p');
		description.textContent = graphDocument.description;
		const authoredIntro = Array.from(owner.childNodes).filter(
			(node): boolean => !(node instanceof HTMLScriptElement && node.type === 'application/json'),
		);
		header.append(title, description, ...authoredIntro);
		this.main.className = 'ex-explorer-main';
		this.heading.className = 'ex-explorer-heading';
		this.breadcrumb.className = 'ex-explorer-history';
		this.breadcrumb.setAttribute('aria-label', 'Selection history');
		this.overview.className = 'ex-explorer-overview';
		this.controls.className = 'ex-explorer-controls';
		this.viewport.className = 'ex-explorer-viewport';
		this.viewport.tabIndex = 0;
		this.viewport.setAttribute('aria-label', 'Composition graph');
		registerInspectionCopy(this.viewport, (): HTMLElement => {
			const copy = document.createElement('div');
			copy.className = this.viewport.className;
			copy.tabIndex = 0;
			copy.setAttribute('aria-label', 'Composition graph');
			this.drawGraph(copy, true);
			return copy;
		});
		this.related.className = 'ex-explorer-related';
		this.tablist.className = 'ex-explorer-tabs';
		this.tablist.setAttribute('role', 'tablist');
		this.tablist.setAttribute('aria-label', 'Explorer details');
		this.panel.className = 'ex-explorer-panel';
		this.panel.setAttribute('role', 'tabpanel');
		this.panel.id = `${owner.id}-details`;
		this.sources.className = 'ex-explorer-sources';
		this.sources.hidden = true;
		const zoom = document.createElement('ex-zoom');
		zoom.setAttribute('label', 'Composition graph');
		zoom.setAttribute('zoom', '');
		zoom.append(this.viewport);
		const note = document.createElement('p');
		note.className = 'ex-explorer-caption';
		note.textContent = graphDocument.note;
		this.main.append(
			header,
			this.overview,
			this.heading,
			this.breadcrumb,
			this.controls,
			zoom,
			this.related,
			this.tablist,
			this.panel,
			this.sources,
			note,
		);
		const shell = document.createElement('div');
		shell.className = 'ex-explorer-shell';
		shell.append(this.browser, this.main);
		owner.replaceChildren(shell);
		this.resizeObserver = new ResizeObserver((): void => {
			if (this.viewport.clientWidth === this.graphWidth || !owner.isConnected) return;
			this.renderGraph();
		});
		this.resizeObserver.observe(this.viewport);
		this.renderBrowser();
		this.render();
		bindTabKeyboard(this.tablist);
		performance.mark(`demistify:explorer:${owner.id}:ready`);
		owner.setAttribute('data-ready', '');
		owner.removeAttribute('aria-busy');
		owner.dispatchEvent(new CustomEvent('island-ready', { bubbles: true }));
	}
	resume(): void {
		this.resizeObserver.observe(this.viewport);
		this.renderGraph();
	}
	dispose(): void {
		this.pendingSearch.cancel();
		this.resizeObserver.disconnect();
	}
	private readonly select = (id: string, pushHistory = true): void => {
		const selectionStart = performance.now();
		if (id !== '__issues' && !this.nodeById.has(id)) return;
		if (pushHistory && id !== this.selection) this.history.push(this.selection);
		this.selection = id;
		this.selectedEdge = '';
		this.view = compositionView(this.graphDocument, id, this.mode);
		const pageUrl = new URL(location.href);
		pageUrl.searchParams.set(`ex.${this.owner.id}`, id);
		history.replaceState(null, '', pageUrl);
		this.renderBrowser();
		this.render();
		performance.measure('demistify:explorer:selection-sync', {
			start: selectionStart,
			end: performance.now(),
		});
		requestAnimationFrame((): void => {
			requestAnimationFrame((): void => {
				performance.measure('demistify:explorer:selection-paint', {
					start: selectionStart,
					end: performance.now(),
				});
			});
		});
	};
	private renderBrowser(): void {
		const previous = new Map(
			[...this.list.querySelectorAll('details')].map((element) => [
				element.dataset['group'],
				element.open,
			]),
		);
		this.list.replaceChildren();
		const query = this.search.value.trim().toLocaleLowerCase();
		for (const group of this.graphDocument.groups) {
			const nodes = this.graphDocument.nodes.filter(
				(node): boolean =>
					node.group === group.id &&
					`${node.label} ${node.description}`.toLocaleLowerCase().includes(query),
			);
			if (!nodes.length) continue;
			const details = document.createElement('details');
			details.dataset['group'] = group.id;
			details.open = Boolean(query) || previous.get(group.id) !== false;
			const summary = document.createElement('summary');
			summary.textContent = `${group.label} · ${nodes.length}`;
			details.append(summary);
			for (const node of nodes.toSorted((leftNode, rightNode): number =>
				leftNode.label.localeCompare(rightNode.label),
			)) {
				const button = document.createElement('button');
				button.type = 'button';
				button.className = `ex-explorer-${group.kind}`;
				button.textContent = node.label;
				button.setAttribute('aria-label', node.label);
				const kindLabel = document.createElement('small');
				kindLabel.textContent = group.label;
				button.append(kindLabel);
				button.title = node.description;
				button.setAttribute('aria-current', String(node.id === this.selection));
				button.addEventListener('click', (): void => this.select(node.id));
				details.append(button);
			}
			this.list.append(details);
		}
		if (!this.list.children.length) this.list.textContent = 'No matching nodes.';
	}
	private renderGraph(): void {
		this.graphWidth = this.viewport.clientWidth;
		this.drawGraph(this.viewport);
	}
	private drawGraph(viewport: HTMLElement, inspected = false): void {
		renderExplorerGraph({
			viewport,
			document: this.graphDocument,
			view: this.view,
			onSelect: (id: string): void => {
				this.select(id);
				if (inspected) this.drawGraph(viewport, true);
			},
			onConnection: (id: string): void => {
				this.selectedEdge = id;
				this.activeTab = 'connections';
				this.renderTabs();
				const row = [...this.panel.querySelectorAll<HTMLElement>('[data-edge-id]')].find(
					(element): boolean => element.dataset['edgeId'] === id,
				);
				row?.scrollIntoView({ block: 'nearest' });
			},
		});
	}
	private render(): void {
		const node = this.nodeById.get(this.selection);
		this.heading.replaceChildren();
		const title = document.createElement('h2');
		title.textContent = node?.label ?? 'Composition issues';
		const description = document.createElement('p');
		description.textContent = node?.description ?? 'Nodes with declared composition gaps.';
		const status = document.createElement('small');
		status.textContent = `${this.view.nodes.length} nodes · ${this.view.edges.length} connections · ${this.mode === 'current' ? 'CURRENT' : 'WITH PROPOSALS'}${node?.issue ? ` · ${node.issue}` : ''}`;
		this.heading.append(title, description, status);
		this.breadcrumb.replaceChildren(
			createIconButton('arrow-left', 'Back to previous selection', (): void => {
				const previous = this.history.pop();
				if (previous) this.select(previous, false);
			}),
		);
		const back = this.breadcrumb.querySelector('button');
		if (back) back.disabled = !this.history.length;
		for (const id of this.history.slice(-3)) {
			const button = document.createElement('button');
			button.type = 'button';
			button.textContent = this.nodeById.get(id)?.label ?? 'Issues';
			button.addEventListener('click', (): void => this.select(id));
			this.breadcrumb.append(button, document.createTextNode(' › '));
		}
		this.breadcrumb.append(document.createTextNode(node?.label ?? 'Issues'));
		this.controls.replaceChildren();
		const modes = createModeChoices<ExplorerMode>(
			'Graph composition',
			[
				{ value: 'current', label: 'Current', icon: 'file' },
				{ value: 'proposed', label: 'With proposals', icon: 'git-branch' },
			],
			this.mode,
			(mode): void => {
				this.mode = mode;
				this.select(this.selection, false);
				this.controls.querySelector<HTMLButtonElement>('[aria-pressed="true"]')?.focus();
			},
		);
		this.controls.append(modes.element);
		const sourceButton = document.createElement('button');
		sourceButton.type = 'button';
		labelControl(sourceButton, 'file', 'Sources & details');
		this.sources.id = `${this.owner.id}-sources`;
		sourceButton.setAttribute('aria-controls', this.sources.id);
		sourceButton.setAttribute('aria-expanded', String(!this.sources.hidden));
		sourceButton.addEventListener('click', (): void => {
			this.sources.hidden = !this.sources.hidden;
			sourceButton.setAttribute('aria-expanded', String(!this.sources.hidden));
		});
		this.controls.append(sourceButton);
		const legend = document.createElement('span');
		legend.className = 'ex-explorer-legend';
		legend.innerHTML = '<span>Current connection</span><span>Proposed connection</span>';
		this.controls.append(legend);
		this.overview.replaceChildren();
		for (const table of this.graphDocument.tables.filter(
			(candidate): boolean => candidate.placement === 'overview',
		))
			this.overview.append(
				supplementaryTable({
					table,
					selection: this.selection,
					nodes: this.nodeById,
					select: this.select,
				}),
			);
		this.renderGraph();
		this.renderTabs();
		this.renderSources();
		this.related.replaceChildren();
		const summary = document.createElement('summary');
		summary.textContent = `Related context · ${this.view.related.length}`;
		this.related.append(summary);
		for (const edge of this.view.related) {
			const button = document.createElement('button');
			button.type = 'button';
			button.textContent = this.nodeById.get(edge.target)?.label ?? edge.target;
			button.addEventListener('click', (): void => this.select(edge.target));
			this.related.append(button);
		}
	}
	private renderTabs(): void {
		const tables = this.graphDocument.tables.filter(
			(table): boolean => table.placement === 'details',
		);
		const tabs = [
			{ id: 'connections', label: `Connections · ${this.view.edges.length}`, icon: 'git-branch' },
			{ id: 'used-by', label: 'Used by', icon: 'plug' },
			...tables.map((table) => ({
				id: table.id,
				label: table.label ?? table.title,
				icon: table.icon ?? 'file',
			})),
		];
		this.tablist.replaceChildren();
		this.panel.replaceChildren();
		for (const tab of tabs) {
			const button = document.createElement('button');
			button.type = 'button';
			labelControl(button, tab.icon, tab.label);
			button.setAttribute('role', 'tab');
			button.id = `${this.owner.id}-tab-${tab.id}`;
			button.setAttribute('aria-controls', this.panel.id);
			if (tab.id === this.activeTab) this.panel.setAttribute('aria-labelledby', button.id);
			button.setAttribute('aria-selected', String(tab.id === this.activeTab));
			button.tabIndex = tab.id === this.activeTab ? 0 : -1;
			button.addEventListener('click', (): void => {
				this.activeTab = tab.id;
				this.renderTabs();
				this.tablist.querySelector<HTMLButtonElement>('[aria-selected="true"]')?.focus();
			});
			this.tablist.append(button);
		}
		if (this.activeTab === 'connections' || this.activeTab === 'used-by') {
			const incoming = this.activeTab === 'used-by';
			const edges = incoming
				? this.graphDocument.edges.filter(
						(edge): boolean =>
							edge.target === this.selection &&
							(this.mode === 'proposed' || edge.status === 'current'),
					)
				: this.view.edges;
			this.panel.append(
				connectionTable({
					edges,
					nodeById: this.nodeById,
					selectedEdge: this.selectedEdge,
					incoming,
					select: this.select,
				}),
			);
		} else {
			const table = tables.find((candidate): boolean => candidate.id === this.activeTab);
			if (table)
				this.panel.append(
					supplementaryTable({
						table,
						selection: this.selection,
						nodes: this.nodeById,
						select: this.select,
					}),
				);
		}
	}
	private renderSources(): void {
		this.sources.replaceChildren();
		const node = this.nodeById.get(this.selection);
		const heading = document.createElement('h3');
		heading.textContent = node?.label ?? 'Composition issues';
		this.sources.append(heading);
		if (node) appendSources(this.sources, node.sources);
		for (const edge of this.graphDocument.edges.filter(
			(candidate): boolean =>
				candidate.source === this.selection &&
				(this.mode === 'proposed' || candidate.status === 'current'),
		)) {
			const title = document.createElement('h4');
			title.textContent = `${edge.label} → ${this.nodeById.get(edge.target)?.label}`;
			this.sources.append(title);
			appendSources(this.sources, edge.sources);
		}
	}
}

export async function mountExplorer(
	owner: HTMLElement,
): Promise<{ readonly controller: ExplorerController; readonly source: string }> {
	const sourcePath = owner.getAttribute('src');
	let input: unknown;
	if (sourcePath) {
		const response = await fetch(new URL(sourcePath, document.baseURI));
		if (!response.ok) throw new Error(`Explorer source failed: HTTP ${response.status}.`);
		input = await response.json();
	} else {
		const inline = owner.querySelector('script[type="application/json"]');
		if (!inline) throw new Error('ex-explorer needs src or an application/json block.');
		input = JSON.parse(inline.textContent ?? '');
	}
	try {
		return {
			controller: new ExplorerController(owner, parseGraphDocument(input)),
			source: JSON.stringify(input),
		};
	} catch (error) {
		reportIssue(owner, error instanceof Error ? error.message : 'Invalid explorer source.');
		throw error;
	}
}
