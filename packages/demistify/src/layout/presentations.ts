import { createIconButton, createPhosphorIcon } from '../core/icons.ts';

interface PresentationMenu {
	readonly button: HTMLButtonElement;
	readonly panel: HTMLElement;
}

/** Local picture targets get a gallery glyph; document and cross-page paths use a path glyph. */
function walkthroughIcon(presentation: HTMLElement): 'images' | 'path' {
	for (const step of presentation.querySelectorAll(':scope > ex-step')) {
		const reference = step.getAttribute('to') ?? '';
		let target: Element | null = document.getElementById(reference);
		if (!target) {
			for (const source of document.querySelectorAll(
				'ex-gallery > template[data-gallery-source]',
			)) {
				if (source instanceof HTMLTemplateElement) {
					target = source.content.getElementById(reference);
					if (target) break;
				}
			}
		}
		if (target?.matches('img, figure, ex-gallery') || target?.closest('ex-gallery'))
			return 'images';
	}
	return 'path';
}

/** Named explainers may be local presentations or ordinary relative links to other documents. */
export function createPresentationMenu(navigation: HTMLElement): PresentationMenu | undefined {
	const authored = Array.from(
		navigation.querySelectorAll<HTMLAnchorElement>('[slot="presentations"] a[href]'),
	);
	const presentations = Array.from(document.querySelectorAll<HTMLElement>('ex-presentation'));
	if (presentations.length === 0 && authored.length === 0) return undefined;
	const panel = document.createElement('nav');
	panel.className = 'ex-presentations-panel';
	panel.popover = 'auto';
	panel.setAttribute('aria-label', 'Walkthroughs');
	const title = document.createElement('strong');
	title.textContent = 'Walkthroughs';
	const close = createIconButton('x', 'Close walkthroughs', (): void => panel.hidePopover());
	const header = document.createElement('header');
	header.append(title, close);
	panel.append(header);
	const links: HTMLAnchorElement[] = [];
	const pictureIcons: { presentation: HTMLElement; icon: HTMLElement }[] = [];
	for (const presentation of presentations) {
		if (!presentation.id) continue;
		const link = document.createElement('a');
		link.href = `#${presentation.id}`;
		const name = document.createElement('strong');
		name.textContent = presentation.getAttribute('title') ?? presentation.id;
		link.addEventListener('click', (event: MouseEvent): void => {
			event.preventDefault();
			presentation.dispatchEvent(new Event('presentation-start'));
			panel.hidePopover();
		});
		const description = document.createElement('span');
		description.textContent = presentation.getAttribute('description') ?? '';
		const copy = document.createElement('div');
		copy.className = 'ex-presentation-copy';
		copy.append(name, description);
		const icon = createPhosphorIcon(walkthroughIcon(presentation));
		pictureIcons.push({ presentation, icon });
		link.append(icon, copy);
		links.push(link);
	}
	for (const link of authored) {
		const copy = link.cloneNode(true);
		if (copy instanceof HTMLAnchorElement) {
			const text = document.createElement('div');
			text.className = 'ex-presentation-copy';
			text.append(...copy.childNodes);
			copy.append(createPhosphorIcon('path'), text);
			links.push(copy);
		}
	}
	panel.append(...links);
	const button = createIconButton('path', 'Browse walkthroughs', (): void => {
		if (panel.matches(':popover-open')) panel.hidePopover();
		else {
			for (const { presentation, icon } of pictureIcons)
				icon.className = `ph ph-${walkthroughIcon(presentation)}`;
			panel.showPopover();
		}
	});
	button.classList.add('ex-presentations-trigger');
	panel.addEventListener('click', (event: MouseEvent): void => {
		if (event.target instanceof Element && event.target.closest('a')) panel.hidePopover();
	});
	const updateCurrent = (): void => {
		const selected = document.getElementById(
			document.documentElement.dataset['presentation'] ?? '',
		);
		for (const link of links) {
			if (selected && link.getAttribute('href') === `#${selected.id}`)
				link.setAttribute('aria-current', 'true');
			else link.removeAttribute('aria-current');
		}
	};
	document.addEventListener('presentation-change', updateCurrent);
	updateCurrent();
	document.body.append(panel);
	return { button, panel };
}
