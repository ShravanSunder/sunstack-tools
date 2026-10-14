import { createIconButton } from './icon-controls.ts';

type InspectionMode = 'modal' | 'sidebar';

const installedInspectors = new WeakSet<HTMLElement>();

export function installIslandInspector(owner: HTMLElement, always: boolean = false): void {
	const zoom = owner.getAttribute('zoom');
	if (
		(!always && (zoom === null || zoom === 'none')) ||
		owner.hasAttribute('data-inspector-content') ||
		installedInspectors.has(owner)
	)
		return;
	installedInspectors.add(owner);
	owner.classList.add('ex-inspectable');
	const mode: InspectionMode =
		owner.getAttribute('zoom') === 'sidebar' || owner.getAttribute('mode') === 'sidebar'
			? 'sidebar'
			: 'modal';
	const label =
		owner.getAttribute('label') ??
		owner.dataset['title'] ??
		owner.querySelector('h1,h2,h3')?.textContent ??
		'Island';
	const trigger = createIconButton(
		mode === 'sidebar' ? 'sidebar-simple' : 'arrows-out',
		`${mode === 'sidebar' ? 'Open sidebar for' : 'Expand'} ${label}`,
		(): void => {
			const originalStyle = owner.getAttribute('style');
			const bounds = owner.getBoundingClientRect();
			const contentNodes = Array.from(owner.childNodes).filter((node): boolean => node !== trigger);
			const detailStates = Array.from(
				owner.querySelectorAll<HTMLElement>(':scope > [slot="details"]'),
			).map((element) => ({ element, hidden: element.hidden }));
			const disclosureStates = Array.from(owner.querySelectorAll('details')).map((element) => ({
				element,
				open: element.open,
			}));
			const dialog = document.createElement('dialog');
			dialog.className = 'ex-inspect-dialog';
			dialog.dataset['mode'] = mode;
			dialog.setAttribute('aria-label', label);
			const header = document.createElement('header');
			header.className = 'ex-inspect-header';
			const title = document.createElement('strong');
			title.textContent = label;
			const body = document.createElement('div');
			body.className = 'ex-inspect-body';
			const content = owner.cloneNode(false);
			if (!(content instanceof HTMLElement)) return;
			content.removeAttribute('id');
			content.removeAttribute('style');
			content.removeAttribute('zoom');
			content.removeAttribute('mode');
			content.classList.remove('ex-inspectable');
			content.setAttribute('data-inspector-content', '');
			let scale = 1;
			const magnification = document.createElement('span');
			magnification.textContent = '100%';
			const addButton = (
				iconName: string,
				buttonLabel: string,
				action: () => void,
			): HTMLButtonElement => {
				const button = createIconButton(iconName, buttonLabel, action);
				header.append(button);
				return button;
			};
			const updateScale = (nextScale: number): void => {
				scale = Math.max(0.5, Math.min(2.5, nextScale));
				content.style.zoom = String(scale);
				magnification.textContent = `${Math.round(scale * 100)}%`;
			};
			header.append(title);
			addButton('magnifying-glass-minus', 'Zoom out', (): void => updateScale(scale - 0.25));
			header.append(magnification);
			addButton('magnifying-glass-plus', 'Zoom in', (): void => updateScale(scale + 0.25));
			addButton('arrow-counter-clockwise', 'Reset zoom', (): void => updateScale(1));
			const closeButton = addButton('x', 'Close inspector', (): void => dialog.close());
			closeButton.autofocus = true;
			owner.style.width = `${bounds.width}px`;
			owner.style.height = `${bounds.height}px`;
			content.append(...contentNodes);
			body.append(content);
			dialog.append(header, body);
			dialog.addEventListener(
				'close',
				(): void => {
					for (const node of contentNodes) owner.insertBefore(node, trigger);
					for (const detail of detailStates) detail.element.hidden = detail.hidden;
					for (const disclosure of disclosureStates) disclosure.element.open = disclosure.open;
					if (originalStyle === null) owner.removeAttribute('style');
					else owner.setAttribute('style', originalStyle);
					dialog.remove();
					trigger.focus();
					document.dispatchEvent(new Event('island-ready'));
				},
				{ once: true },
			);
			document.body.append(dialog);
			for (const detail of detailStates) detail.element.hidden = false;
			for (const disclosure of disclosureStates) disclosure.element.open = true;
			dialog.showModal();
		},
	);
	trigger.classList.add('ex-inspect-trigger');
	owner.append(trigger);
}

export class ZoomIsland extends HTMLElement {
	connectedCallback(): void {
		installIslandInspector(this, true);
		for (const detail of this.querySelectorAll<HTMLElement>(':scope > [slot="details"]'))
			detail.hidden = true;
	}
}
