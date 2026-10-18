import { createDebouncedAction } from '../core/debounce.ts';
import { reportIssue } from '../core/issues.ts';
import { anchorPopover } from '../core/popover-position.ts';
import {
	filterGalleryCaptures,
	galleryScreenKey,
	readGalleryCaptures,
	readGalleryFacets,
	type GalleryCapture,
	type GalleryFacet,
} from './gallery-data.ts';
import {
	createGalleryControls,
	renderGalleryFacetControls,
	galleryElement,
	type GalleryControls,
} from './gallery-view.ts';
import { copyInspectionContent, registerInspectionCopy } from './inspection-copy.ts';
import { installInspectionGestures } from './inspection-gestures.ts';
import { openIsland } from './inspector.ts';

type GalleryView = 'review' | 'grid';
type ZoomMode = 'fit' | 'fill' | 'manual';
export interface GallerySnapshot {
	readonly view: GalleryView;
	readonly selected: string | undefined;
	readonly query: string;
	readonly zoomMode: ZoomMode;
	readonly scale: number;
	readonly scrollLeft: number;
	readonly scrollTop: number;
	readonly filters: ReadonlyMap<string, ReadonlySet<string>>;
}

/** Agent-authored evidence; document chrome and inspection remain owned by the shared library. */
export class GalleryIsland extends HTMLElement {
	private static activeGallery: GalleryIsland | undefined;
	private sourceTemplate: HTMLTemplateElement | undefined;
	private captures: readonly GalleryCapture[] = [];
	private facets: readonly GalleryFacet[] = [];
	private visible: readonly GalleryCapture[] = [];
	private filters = new Map<string, Set<string>>();
	private selected: string | undefined;
	private view: GalleryView = 'review';
	private controls: GalleryControls | undefined;
	private activeFigure: HTMLElement | undefined;
	private missingView: HTMLElement | undefined;
	private activeImage: HTMLImageElement | undefined;
	private imageStage: HTMLElement | undefined;
	private renderedSelection: string | undefined;
	private renderedList = '';
	private scale = 1;
	private zoomMode: ZoomMode = 'fit';
	private resizeObserver: ResizeObserver | undefined;
	private initialSnapshot: GallerySnapshot | undefined;
	private inspectionSource: GalleryIsland | undefined;
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
		if (GalleryIsland.activeGallery === this && this.inspectionSource?.isConnected)
			GalleryIsland.activeGallery = this.inspectionSource;
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
		const snapshot = this.initialSnapshot;
		this.view = snapshot?.view ?? (this.getAttribute('view') === 'grid' ? 'grid' : 'review');
		this.selected =
			snapshot?.selected ?? this.selected ?? this.getAttribute('selected') ?? this.captures[0]?.id;
		if (snapshot)
			for (const [field, values] of snapshot.filters) this.filters.set(field, new Set(values));
		const controls = createGalleryControls(this.id);
		this.controls = controls;
		controls.search.value = snapshot?.query ?? '';
		controls.searchField.sync();
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
		if (!GalleryIsland.activeGallery?.isConnected) GalleryIsland.activeGallery = this;
		this.installControls(controls);
		renderGalleryFacetControls({
			controls,
			captures: this.captures,
			facets: this.facets,
			reset: (): void => {
				this.pendingFilter.cancel();
				controls.search.value = '';
				controls.searchField.sync();
				for (const values of this.filters.values()) values.clear();
				this.applyFilters();
			},
			change: (field, value, checked): void => {
				const chosen = this.filters.get(field);
				if (checked) chosen?.add(value);
				else chosen?.delete(value);
				this.pendingFilter.schedule();
			},
		});
		this.renderedSelection = undefined;
		this.renderedList = '';
		this.applyFilters();
		if (snapshot) this.restoreBrowseSnapshot(snapshot);
		registerInspectionCopy(this, (): HTMLElement => {
			const copy = document.createElement('ex-gallery');
			if (!(copy instanceof GalleryIsland)) throw new Error('Gallery is not registered.');
			copy.setAttribute('label', this.getAttribute('label') ?? 'Screenshot gallery');
			copy.inspectionSource = this;
			// Capture IDs live inside an inert template, outside the outer copy's normal ID walk.
			const frame = document.createElement('div');
			frame.append(source.content.cloneNode(true));
			const content = copyInspectionContent(frame);
			if (!content) throw new Error('Gallery source could not be copied.');
			const captures = readGalleryCaptures(content, `gallery-copy-${crypto.randomUUID()}`);
			for (const capture of captures) capture.source.id = capture.id;
			const copiedState = this.browseSnapshot();
			const selectedIndex = this.captures.findIndex(
				(capture): boolean => capture.id === copiedState.selected,
			);
			copy.initialSnapshot = { ...copiedState, selected: captures[selectedIndex]?.id };
			const template = document.createElement('template');
			template.dataset['gallerySource'] = '';
			template.content.append(...content.childNodes);
			copy.append(template);
			return copy;
		});
		this.dispatchEvent(new Event('island-ready', { bubbles: true }));
	}

	private installControls(controls: GalleryControls): void {
		anchorPopover(controls.filtersButton, controls.filtersPanel, this.eventController.signal);
		anchorPopover(controls.help, controls.helpPanel, this.eventController.signal);
		controls.search.addEventListener('input', (): void => {
			this.pendingFilter.schedule();
		});
		controls.reviewChoice.addEventListener('click', (): void => this.changeView('review'));
		controls.gridChoice.addEventListener('click', (): void => this.changeView('grid'));
		controls.fit.addEventListener('click', (): void => {
			this.zoomMode = 'fit';
			this.fitImage();
		});
		controls.fill.addEventListener('click', (): void => {
			this.zoomMode = 'fill';
			this.fitImage();
		});
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
		const activate = (): void => {
			GalleryIsland.activeGallery = this;
		};
		this.addEventListener('focusin', activate, { signal: this.eventController.signal });
		this.addEventListener('pointerdown', activate, { signal: this.eventController.signal });
		document.addEventListener(
			'keydown',
			(event: KeyboardEvent): void => {
				if (
					GalleryIsland.activeGallery !== this ||
					event.defaultPrevented ||
					document.documentElement.hasAttribute('data-presentation') ||
					event.ctrlKey ||
					event.metaKey ||
					event.altKey ||
					!(event.target instanceof Element) ||
					event.target.closest(
						'input,textarea,select,[contenteditable="true"],[popover],dialog,.ex-drawer,[role="tab"],[role="separator"]',
					)
				)
					return;
				if (document.querySelector(':popover-open')) return;
				if (
					!(
						event.target === document.body ||
						event.target === document.documentElement ||
						this.contains(event.target)
					)
				)
					return;
				const focused = this.ownerDocument.activeElement;
				const wasPreview = focused?.closest('.ex-gallery-preview');
				if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
					event.preventDefault();
					this.moveCapture(event.key === 'ArrowLeft' ? -1 : 1);
				} else if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
					event.preventDefault();
					this.moveViewport(event.key === 'ArrowUp' ? -1 : 1);
				}
				if (focused instanceof HTMLElement && !focused.isConnected) {
					const replacement = wasPreview
						? this.imageStage
						: this.querySelector<HTMLElement>(
								`[data-capture-id="${CSS.escape(this.selected ?? '')}"]`,
							);
					replacement?.focus({ preventScroll: true });
				}
			},
			{ signal: this.eventController.signal },
		);
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
		controls.reviewChoice.setAttribute('aria-pressed', String(this.view === 'review'));
		controls.reviewChoice.tabIndex = this.view === 'review' ? 0 : -1;
		controls.gridChoice.setAttribute('aria-pressed', String(this.view === 'grid'));
		controls.gridChoice.tabIndex = this.view === 'grid' ? 0 : -1;
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
			button.addEventListener('click', (event: MouseEvent): void => {
				event.stopPropagation();
				this.selectCapture(capture.id);
			});
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
		this.missingView?.remove();
		if (this.activeFigure) this.activeFigure.hidden = false;
		for (const group of controls.toolbar.querySelectorAll<HTMLElement>('.ex-gallery-control-group'))
			group.inert = false;
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
			this.zoomMode !== 'manual' ? this.fitImage() : this.changeScale(this.scale, this.zoomMode),
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
			if (this.zoomMode !== 'manual') this.fitImage();
		});
		this.resizeObserver.observe(stage);
		if (image.complete && image.naturalWidth > 0) {
			if (this.zoomMode !== 'manual') this.fitImage();
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
		const widthRatio =
			(this.imageStage.clientWidth - horizontalPadding) / this.activeImage.naturalWidth;
		const heightRatio =
			(Math.floor(this.imageStage.getBoundingClientRect().height) - verticalPadding) /
			this.activeImage.naturalHeight;
		const fill = this.zoomMode === 'fill';
		const scale = fill ? Math.max(widthRatio, heightRatio) : Math.min(1, widthRatio, heightRatio);
		this.changeScale(Math.max(0.02, scale), fill ? 'fill' : 'fit');
	}

	private changeScale(scale: number, mode: ZoomMode): void {
		this.scale = Math.min(4, Math.max(0.02, scale));
		this.zoomMode = mode;
		if (this.activeImage?.naturalWidth) {
			this.activeImage.style.width = `${this.activeImage.naturalWidth * this.scale}px`;
			this.activeImage.style.height = `${this.activeImage.naturalHeight * this.scale}px`;
		}
		if (this.controls) {
			this.controls.fit.setAttribute('aria-pressed', String(mode === 'fit'));
			this.controls.fill.setAttribute('aria-pressed', String(mode === 'fill'));
		}
	}

	private changeView(view: GalleryView): void {
		if (view === this.view) return;
		this.pendingFilter.cancel();
		this.view = view;
		this.renderedSelection = undefined;
		this.activeFigure?.remove();
		this.activeFigure = undefined;
		this.applyFilters();
	}

	private selectCapture(id: string): void {
		this.selected = id;
		this.renderPreview();
		this.updateNavigation();
	}

	private moveCapture(direction: number): void {
		const current = this.visible.find((capture): boolean => capture.id === this.selected);
		if (!current) return;
		const keys = [...new Set(this.visible.map(galleryScreenKey))];
		const next = keys[keys.indexOf(galleryScreenKey(current)) + direction];
		if (next === undefined) return;
		const candidates = this.visible.filter(
			(capture): boolean => galleryScreenKey(capture) === next,
		);
		const chosen =
			candidates.find(
				(capture): boolean => capture.metadata['viewport'] === current.metadata['viewport'],
			) ?? candidates[0];
		if (!chosen) return;
		this.selectCapture(chosen.id);
		this.controls?.strip
			.querySelector(`[data-capture-id="${CSS.escape(chosen.id)}"]`)
			?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
	}

	private moveViewport(direction: number): void {
		const current = this.visible.find((capture): boolean => capture.id === this.selected);
		if (!current) return;
		const candidates = this.visible.filter(
			(capture): boolean => galleryScreenKey(capture) === galleryScreenKey(current),
		);
		const next = candidates[candidates.indexOf(current) + direction];
		if (next) this.selectCapture(next.id);
	}

	private updateNavigation(): void {
		const controls = this.controls;
		if (!controls) return;

		for (const button of controls.strip.querySelectorAll<HTMLElement>('[data-capture-id]'))
			button.setAttribute('aria-current', String(button.dataset['captureId'] === this.selected));
	}

	/** Playback borrows the gallery; the gallery remains the owner of browse and zoom state. */
	browseSnapshot(): GallerySnapshot {
		return {
			view: this.view,
			selected: this.selected,
			query: this.controls?.search.value ?? '',
			filters: new Map([...this.filters].map(([field, values]) => [field, new Set(values)])),
			zoomMode: this.zoomMode,
			scale: this.scale,
			scrollLeft: this.imageStage?.scrollLeft ?? 0,
			scrollTop: this.imageStage?.scrollTop ?? 0,
		};
	}

	restoreBrowseSnapshot(snapshot: GallerySnapshot): void {
		this.pendingFilter.cancel();
		this.view = snapshot.view;
		this.selected = snapshot.selected;
		this.filters = new Map(
			[...snapshot.filters].map(([field, values]) => [field, new Set(values)]),
		);
		this.zoomMode = snapshot.zoomMode;
		this.scale = snapshot.scale;
		if (this.controls) {
			this.controls.search.value = snapshot.query;
			this.controls.searchField.sync();
		}
		this.renderedSelection = undefined;
		this.applyFilters();
		const stage = this.imageStage;
		const image = this.activeImage;
		const restoreScroll = (): void => stage?.scrollTo(snapshot.scrollLeft, snapshot.scrollTop);
		if (image?.complete) restoreScroll();
		else image?.addEventListener('load', restoreScroll, { once: true });
	}

	prepareWalkthrough(): GallerySnapshot {
		const snapshot = this.browseSnapshot();
		this.restoreBrowseSnapshot({
			...snapshot,
			view: 'review',
			query: '',
			filters: new Map(),
			zoomMode: 'fit',
			scale: 1,
			scrollLeft: 0,
			scrollTop: 0,
		});
		return snapshot;
	}

	walkthroughCaptures(): readonly GalleryCapture[] {
		return this.captures;
	}

	showUnavailableCapture(title: string, description: string): HTMLElement {
		this.missingView ??= galleryElement('section', 'ex-gallery-gap');
		this.missingView.setAttribute('role', 'status');
		this.missingView.setAttribute('aria-label', title);
		this.missingView.replaceChildren(
			galleryElement('strong', '', title),
			galleryElement('p', '', description),
		);
		if (this.activeFigure) this.activeFigure.hidden = true;
		if (this.controls) {
			this.controls.title.textContent = title;
			this.controls.viewer.append(this.missingView);
			for (const group of this.controls.toolbar.querySelectorAll<HTMLElement>(
				'.ex-gallery-control-group',
			))
				group.inert = true;
		}
		return this.missingView;
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
