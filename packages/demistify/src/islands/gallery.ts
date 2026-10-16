import { createDebouncedAction } from '../core/debounce.ts';
import { createPhosphorIcon } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import {
	filterGalleryCaptures,
	readGalleryCaptures,
	readGalleryFacets,
	type GalleryCapture,
	type GalleryFacet,
} from './gallery-data.ts';
import { createGalleryControls, galleryElement, type GalleryControls } from './gallery-view.ts';
import { registerInspectionCopy } from './inspection-copy.ts';
import { installInspectionGestures } from './inspection-gestures.ts';
import { openIsland } from './inspector.ts';

type GalleryView = 'review' | 'grid';
type ZoomMode = 'fit' | 'natural' | 'manual';
interface GallerySnapshot {
	readonly view: GalleryView;
	readonly selected: string | undefined;
	readonly query: string;
	readonly filters: ReadonlyMap<string, ReadonlySet<string>>;
}

/** Agent-authored evidence; document chrome and inspection remain owned by the shared library. */
export class GalleryIsland extends HTMLElement {
	private sourceTemplate: HTMLTemplateElement | undefined;
	private captures: readonly GalleryCapture[] = [];
	private facets: readonly GalleryFacet[] = [];
	private visible: readonly GalleryCapture[] = [];
	private filters = new Map<string, Set<string>>();
	private selected: string | undefined;
	private view: GalleryView = 'review';
	private controls: GalleryControls | undefined;
	private activeFigure: HTMLElement | undefined;
	private activeImage: HTMLImageElement | undefined;
	private imageStage: HTMLElement | undefined;
	private renderedSelection: string | undefined;
	private renderedList = '';
	private scale = 1;
	private zoomMode: ZoomMode = 'fit';
	private resizeObserver: ResizeObserver | undefined;
	private initialSnapshot: GallerySnapshot | undefined;
	private eventController = new AbortController();
	private readonly pendingFilter = createDebouncedAction((): void => this.applyFilters(), 200);

	connectedCallback(): void {
		try {
			this.initialize();
		} catch (error) {
			reportIssue(this, `Gallery could not initialize: ${String(error)}`);
		}
	}

	disconnectedCallback(): void {
		this.pendingFilter.cancel();
		this.resizeObserver?.disconnect();
		this.eventController.abort();
	}

	private initialize(): void {
		this.eventController.abort();
		this.eventController = new AbortController();
		this.id ||= `gallery-${crypto.randomUUID()}`;
		const existing = this.querySelector<HTMLTemplateElement>(
			':scope > template[data-gallery-source]',
		);
		const source = this.sourceTemplate ?? existing ?? document.createElement('template');
		if (!this.sourceTemplate && !existing) {
			source.dataset['gallerySource'] = '';
			source.content.append(...Array.from(this.children));
		}
		this.sourceTemplate = source;
		this.captures = readGalleryCaptures(source.content, this.id);
		this.facets = readGalleryFacets(this, this.captures);
		const declaration = source.content.querySelector('script[data-gallery-filters]');
		if (declaration) {
			const container = document.createElement('div');
			container.append(declaration.cloneNode(true));
			this.facets = readGalleryFacets(container, this.captures);
		}
		this.filters = new Map(
			this.facets.map((facet): [string, Set<string>] => [facet.field, new Set()]),
		);
		if (!this.filters.has('viewport')) this.filters.set('viewport', new Set());
		const snapshot = this.initialSnapshot;
		this.view = snapshot?.view ?? (this.getAttribute('view') === 'grid' ? 'grid' : 'review');
		this.selected =
			snapshot?.selected ?? this.selected ?? this.getAttribute('selected') ?? this.captures[0]?.id;
		if (snapshot)
			for (const [field, values] of snapshot.filters) this.filters.set(field, new Set(values));
		const controls = createGalleryControls(this.id);
		this.controls = controls;
		controls.search.value = snapshot?.query ?? '';
		this.replaceChildren(
			source,
			controls.header,
			controls.filtersPanel,
			controls.body,
			controls.grid,
			controls.message,
		);
		this.setAttribute('role', 'region');
		this.setAttribute('aria-label', this.getAttribute('label') ?? 'Screenshot gallery');
		this.installControls(controls);
		this.renderFacetControls(controls);
		this.renderedSelection = undefined;
		this.renderedList = '';
		this.applyFilters();
		registerInspectionCopy(this, (): HTMLElement => {
			const copy = document.createElement('ex-gallery');
			if (!(copy instanceof GalleryIsland)) throw new Error('Gallery is not registered.');
			copy.setAttribute('label', this.getAttribute('label') ?? 'Screenshot gallery');
			copy.initialSnapshot = {
				view: this.view,
				selected: this.selected,
				query: controls.search.value,
				filters: this.filters,
			};
			copy.append(source.cloneNode(true));
			return copy;
		});
		this.dispatchEvent(new Event('island-ready', { bubbles: true }));
	}

	private installControls(controls: GalleryControls): void {
		controls.search.addEventListener('input', (): void => {
			this.pendingFilter.schedule();
		});
		controls.reviewButton.addEventListener('click', (): void => this.changeView('review'));
		controls.gridButton.addEventListener('click', (): void => this.changeView('grid'));
		controls.previous.addEventListener('click', (): void => this.moveCapture(-1));
		controls.next.addEventListener('click', (): void => this.moveCapture(1));
		controls.previousViewport.addEventListener('click', (): void => this.moveViewport(-1));
		controls.nextViewport.addEventListener('click', (): void => this.moveViewport(1));
		controls.viewport.addEventListener('change', (): void =>
			this.chooseViewport(controls.viewport.value),
		);
		controls.fit.addEventListener('click', (): void => {
			this.zoomMode = 'fit';
			this.fitImage();
		});
		controls.natural.addEventListener('click', (): void => this.changeScale(1, 'natural'));
		controls.zoomOut.addEventListener('click', (): void =>
			this.changeScale(this.scale / 1.25, 'manual'),
		);
		controls.zoomIn.addEventListener('click', (): void =>
			this.changeScale(this.scale * 1.25, 'manual'),
		);
		controls.sidebar.addEventListener('click', (): void => {
			this.inspectSelected('sidebar');
		});
		controls.fullscreen.addEventListener('click', (): void => {
			this.inspectSelected('modal');
		});
		this.addEventListener(
			'keydown',
			(event: KeyboardEvent): void => {
				if (
					event.ctrlKey ||
					event.metaKey ||
					event.altKey ||
					!(event.target instanceof Element) ||
					event.target.closest('input,textarea,select,[contenteditable="true"],[popover]')
				)
					return;
				if (controls.filtersPanel.matches(':popover-open')) return;
				if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
					event.preventDefault();
					this.moveCapture(event.key === 'ArrowUp' ? -1 : 1);
				} else if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
					event.preventDefault();
					this.moveViewport(event.key === 'ArrowLeft' ? -1 : 1);
				}
			},
			{ signal: this.eventController.signal },
		);
	}

	private renderFacetControls(controls: GalleryControls): void {
		const reset = galleryElement('button', '', 'Reset filters');
		reset.type = 'button';
		reset.addEventListener('click', (): void => {
			this.pendingFilter.cancel();
			controls.search.value = '';
			for (const values of this.filters.values()) values.clear();
			this.applyFilters();
		});
		controls.filtersPanel.append(reset);
		for (const facet of this.facets) {
			const group = galleryElement('fieldset');
			const legend = galleryElement('legend', '', facet.label);
			if (facet.icon) legend.prepend(createPhosphorIcon(facet.icon));
			group.append(legend);
			for (const value of new Set(
				this.captures.map((capture): string => capture.metadata[facet.field] ?? '').filter(Boolean),
			)) {
				const label = galleryElement('label');
				const input = galleryElement('input');
				input.type = 'checkbox';
				input.value = value;
				input.dataset['galleryFacet'] = facet.field;
				input.addEventListener('change', (): void => {
					const chosen = this.filters.get(facet.field);
					if (input.checked) chosen?.add(value);
					else chosen?.delete(value);
					this.pendingFilter.schedule();
				});
				label.append(input, value);
				group.append(label);
			}
			controls.filtersPanel.append(group);
		}
	}

	private applyFilters(): void {
		const controls = this.controls;
		if (!controls) return;
		this.visible = filterGalleryCaptures(this.captures, controls.search.value, this.filters);
		if (!this.visible.some((capture): boolean => capture.id === this.selected))
			this.selected = this.visible[0]?.id;
		controls.count.textContent = `${this.visible.length} screenshot${this.visible.length === 1 ? '' : 's'}`;
		controls.message.textContent = this.visible.length
			? ''
			: 'No screenshots match. Change or reset the filters.';
		const listKey = `${this.view}:${this.visible.map((capture): string => capture.id).join('|')}`;
		if (listKey !== this.renderedList) {
			this.renderList();
			this.renderedList = listKey;
		}
		this.updateNavigation();
		this.renderPreview();
		for (const input of controls.filtersPanel.querySelectorAll<HTMLInputElement>(
			'input[data-gallery-facet]',
		))
			input.checked =
				this.filters.get(input.dataset['galleryFacet'] ?? '')?.has(input.value) ?? false;
		this.setAttribute('view', this.view);
		controls.reviewButton.setAttribute('aria-pressed', String(this.view === 'review'));
		controls.gridButton.setAttribute('aria-pressed', String(this.view === 'grid'));
		controls.body.hidden = this.view !== 'review' || !this.visible.length;
		controls.grid.hidden = this.view !== 'grid' || !this.visible.length;
		this.dispatchEvent(new Event('gallery-updated', { bubbles: true }));
	}

	private renderList(): void {
		const controls = this.controls;
		if (!controls) return;
		controls.strip.replaceChildren();
		controls.grid.replaceChildren();
		let groupName = '';
		for (const capture of this.visible) {
			const screen = capture.metadata['screen'] ?? 'Captures';
			if (screen !== groupName) {
				controls.strip.append(galleryElement('p', 'ex-gallery-group', screen));
				groupName = screen;
			}
			const button = galleryElement('button', 'ex-gallery-thumbnail');
			button.type = 'button';
			button.dataset['captureId'] = capture.id;
			button.setAttribute('aria-label', capture.title);
			const image = capture.source.querySelector<HTMLImageElement>('img')?.cloneNode(true);
			if (image instanceof HTMLImageElement) {
				image.loading = 'lazy';
				image.alt = '';
				button.append(image);
			}
			const label = galleryElement('span', '', capture.title);
			if (capture.metadata['viewport'])
				label.append(galleryElement('small', '', capture.metadata['viewport']));
			button.append(label);
			button.addEventListener('click', (): void => this.selectCapture(capture.id));
			controls.strip.append(button);
			if (this.view === 'grid') {
				const figure = this.captureFigure(capture);
				const gridImage = figure.querySelector('img');
				const open = galleryElement('button', 'ex-gallery-grid-open');
				open.type = 'button';
				open.setAttribute('aria-label', `Inspect ${capture.title}`);
				if (gridImage) {
					gridImage.replaceWith(open);
					open.append(gridImage);
				}
				open.addEventListener('click', (event: MouseEvent): void => {
					event.stopPropagation();
					this.selected = capture.id;
					openIsland(this.captureFigure(capture), 'sidebar');
					this.updateNavigation();
				});
				controls.grid.append(figure);
			}
		}
	}

	private captureFigure(capture: GalleryCapture): HTMLElement {
		const figure = capture.source.cloneNode(true);
		if (!(figure instanceof HTMLElement)) throw new Error('Invalid gallery figure.');
		for (const controls of figure.querySelectorAll('.ex-inspect-controls')) controls.remove();
		figure.id = capture.id;
		figure.classList.add('ex-gallery-picture');
		figure.setAttribute('label', capture.title);
		return figure;
	}

	private renderPreview(): void {
		const controls = this.controls;
		const capture = this.visible.find((item): boolean => item.id === this.selected);
		if (!controls || !capture || this.view !== 'review') return;
		controls.title.textContent = capture.title;
		controls.original.href = capture.source.querySelector<HTMLImageElement>('img')?.src ?? '';
		if (this.renderedSelection === capture.id) return;
		this.renderedSelection = capture.id;
		this.activeFigure?.remove();
		this.resizeObserver?.disconnect();
		const figure = this.captureFigure(capture);
		const image = figure.querySelector<HTMLImageElement>('img');
		if (!image) return;
		const stage = galleryElement('div', 'ex-gallery-preview');
		stage.tabIndex = 0;
		stage.setAttribute('aria-label', 'Screenshot viewport');
		image.replaceWith(stage);
		stage.append(image);
		image.loading = 'eager';
		image.addEventListener('click', (event: MouseEvent): void => {
			event.stopPropagation();
			this.inspectSelected('sidebar');
		});
		this.activeFigure = figure;
		this.activeImage = image;
		this.imageStage = stage;
		controls.viewer.append(figure);
		image.addEventListener('load', (): void =>
			this.zoomMode === 'fit' ? this.fitImage() : this.changeScale(this.scale, this.zoomMode),
		);
		image.addEventListener('error', (): void => {
			controls.message.textContent =
				'This screenshot could not load. Check its authored image path or URL.';
		});
		installInspectionGestures(stage, {
			readScale: (): number => this.scale,
			setScale: (scale: number): void => this.changeScale(scale, 'manual'),
		});
		this.resizeObserver = new ResizeObserver((): void => {
			if (this.zoomMode === 'fit') this.fitImage();
		});
		this.resizeObserver.observe(stage);
		if (image.complete && image.naturalWidth > 0) {
			if (this.zoomMode === 'fit') this.fitImage();
			else this.changeScale(this.scale, this.zoomMode);
		}
	}

	private inspectSelected(mode: 'sidebar' | 'modal'): void {
		const capture = this.captures.find((item): boolean => item.id === this.selected);
		if (capture) openIsland(this.captureFigure(capture), mode);
	}

	private fitImage(): void {
		if (!this.activeImage?.naturalWidth || !this.imageStage) return;
		const style = getComputedStyle(this.imageStage);
		const horizontalPadding =
			Number.parseFloat(style.paddingLeft) + Number.parseFloat(style.paddingRight);
		const verticalPadding =
			Number.parseFloat(style.paddingTop) + Number.parseFloat(style.paddingBottom);
		const scale = Math.min(
			1,
			(this.imageStage.clientWidth - horizontalPadding) / this.activeImage.naturalWidth,
			(Math.floor(this.imageStage.getBoundingClientRect().height) - verticalPadding) /
				this.activeImage.naturalHeight,
		);

		this.changeScale(Math.max(0.02, scale), 'fit');
	}

	private changeScale(scale: number, mode: ZoomMode): void {
		this.scale = Math.min(4, Math.max(0.02, scale));
		this.zoomMode = mode;
		if (this.activeImage?.naturalWidth) {
			this.activeImage.style.width = `${this.activeImage.naturalWidth * this.scale}px`;
			this.activeImage.style.height = `${this.activeImage.naturalHeight * this.scale}px`;
		}
		if (this.controls) {
			this.controls.zoom.textContent = `${Math.round(this.scale * 100)}%`;
			this.controls.fit.setAttribute('aria-pressed', String(mode === 'fit'));
			this.controls.natural.setAttribute('aria-pressed', String(mode === 'natural'));
		}
	}

	private changeView(view: GalleryView): void {
		this.pendingFilter.cancel();
		this.view = view;
		this.renderedSelection = undefined;
		this.activeFigure?.remove();
		this.activeFigure = undefined;
		this.applyFilters();
	}

	private selectCapture(id: string): void {
		this.pendingFilter.cancel();
		this.selected = id;
		this.renderPreview();
		this.updateNavigation();
	}

	private moveCapture(direction: number): void {
		const index = this.visible.findIndex((capture): boolean => capture.id === this.selected);
		const next = this.visible[index + direction];
		if (!next) return;
		this.selectCapture(next.id);
		this.controls?.strip
			.querySelector(`[data-capture-id="${CSS.escape(next.id)}"]`)
			?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
	}

	private viewportValues(): readonly string[] {
		const filters = new Map(this.filters);
		filters.delete('viewport');
		return Array.from(
			new Set(
				filterGalleryCaptures(this.captures, this.controls?.search.value ?? '', filters)
					.map((capture): string => capture.metadata['viewport'] ?? '')
					.filter(Boolean),
			),
		);
	}

	private chooseViewport(value: string): void {
		this.pendingFilter.cancel();
		const screen = this.captures.find((capture): boolean => capture.id === this.selected)?.metadata[
			'screen'
		];
		this.filters.set('viewport', new Set(value ? [value] : []));
		const candidates = filterGalleryCaptures(
			this.captures,
			this.controls?.search.value ?? '',
			this.filters,
		);
		this.selected =
			candidates.find((capture): boolean => capture.metadata['screen'] === screen)?.id ??
			candidates[0]?.id;
		this.applyFilters();
	}

	private moveViewport(direction: number): void {
		const values = this.viewportValues();
		const current =
			this.filters.get('viewport')?.values().next().value ??
			this.captures.find((capture): boolean => capture.id === this.selected)?.metadata['viewport'];
		const index = values.findIndex((value): boolean => value === current);
		const next = values[index + direction];
		if (next !== undefined) this.chooseViewport(next);
	}

	private updateNavigation(): void {
		const controls = this.controls;
		if (!controls) return;
		const index = this.visible.findIndex((capture): boolean => capture.id === this.selected);
		controls.previous.disabled = index <= 0;
		controls.next.disabled = index < 0 || index >= this.visible.length - 1;
		for (const button of controls.strip.querySelectorAll<HTMLElement>('[data-capture-id]'))
			button.setAttribute('aria-current', String(button.dataset['captureId'] === this.selected));
		const values = this.viewportValues();
		const viewportFilters = this.filters.get('viewport');
		const multiple = (viewportFilters?.size ?? 0) > 1;
		const chosen = multiple ? '' : (viewportFilters?.values().next().value ?? '');
		controls.viewport.replaceChildren(
			new Option('All viewports', ''),
			...values.map((value): HTMLOptionElement => new Option(value, value)),
		);
		if (multiple) {
			const option = new Option('Multiple viewports', '');
			option.disabled = true;
			controls.viewport.prepend(option);
			controls.viewport.selectedIndex = 0;
		} else controls.viewport.value = chosen;
		const current =
			chosen ||
			this.captures.find((capture): boolean => capture.id === this.selected)?.metadata['viewport'];
		const position = values.findIndex((value): boolean => value === current);
		controls.previousViewport.disabled = position <= 0;
		controls.nextViewport.disabled = position < 0 || position >= values.length - 1;
	}

	/** Fragment resolution can select an authored capture without clearing the reader's filters. */
	revealCapture(id: string): HTMLElement | undefined {
		if (!this.captures.some((capture): boolean => capture.id === id)) return undefined;
		if (!this.visible.some((capture): boolean => capture.id === id)) {
			if (this.controls)
				this.controls.message.textContent = 'This capture is excluded by the current filters.';
			return undefined;
		}
		this.view = 'review';
		this.selected = id;
		this.renderedSelection = undefined;
		this.applyFilters();
		return this.activeFigure;
	}
}
