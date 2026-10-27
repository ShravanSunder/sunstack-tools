import { createIconButton } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import { type ChoiceResponse, choiceResponseSchema } from '../core/review-model.ts';
import { checkAttributes } from '../vocabulary.ts';
import { copyInspectionContent, registerInspectionCopy } from './inspection-copy.ts';
import { installIslandInspector, openIsland } from './inspector.ts';

interface RenderedOption {
	readonly optionId: string;
	readonly label: string;
	readonly input: HTMLInputElement;
	readonly target: HTMLElement;
}

export class OptionIsland extends HTMLElement {}

/** Native input nodes own answers; inert author source owns reconstruction. */
export class ChoiceIsland extends HTMLElement {
	private readonly instanceId = `ex-choice-${crypto.randomUUID()}`;
	private source: HTMLTemplateElement | undefined;
	private options: readonly RenderedOption[] = [];
	private questionId = '';
	private question = '';
	private rendering: Promise<void> | undefined;
	private events = new AbortController();
	private copiedResponse: ChoiceResponse | undefined;
	private initialized = false;

	get response(): ChoiceResponse {
		return choiceResponseSchema.parse({
			questionId: this.questionId,
			question: this.question,
			selected: this.options
				.filter((option): boolean => option.input.checked)
				.map((option) => ({ optionId: option.optionId, label: option.label })),
		});
	}

	connectedCallback(): void {
		if (this.hasAttribute('data-inspector-content')) return;
		if (!this.copiedResponse && this.closest('[data-inspector-copy]')) {
			this.copiedResponse = choiceResponseSchema.parse({
				questionId: this.id || this.instanceId,
				question: this.getAttribute('title'),
				selected: [],
			});
		}
		this.rendering ??= this.initialize().catch((error: unknown): void => {
			this.replaceChildren();
			const message = document.createElement('p');
			message.setAttribute('role', 'alert');
			message.textContent = `Choice could not load: ${String(error)}`;
			this.append(message);
			reportIssue(this, message.textContent);
		});
		void this.rendering.then((): void => {
			if (this.isConnected && this.initialized) this.connectControls();
		});
	}

	disconnectedCallback(): void {
		this.events.abort();
	}

	private async initialize(): Promise<void> {
		this.questionId = this.copiedResponse?.questionId ?? this.id.trim();
		this.question = this.copiedResponse?.question ?? this.getAttribute('title')?.trim() ?? '';
		if (!this.questionId || !this.question)
			throw new Error('ex-choice needs an authored id and question title.');
		if (!this.copiedResponse) {
			const problems = checkAttributes(
				'ex-choice',
				new Map(
					[...this.attributes].map((attribute): [string, string] => [
						attribute.name,
						attribute.value,
					]),
				),
				this.parentElement?.localName,
			);
			if (problems.length)
				throw new Error(problems.map((problem): string => problem.message).join(' '));
			if (this.ownerDocument.querySelectorAll(`#${CSS.escape(this.questionId)}`).length > 1)
				throw new Error('Choice IDs must be unique.');
		}
		const retained = this.querySelector<HTMLTemplateElement>(
			':scope > template[data-choice-source]',
		);
		this.source = retained ?? document.createElement('template');
		if (!retained) {
			this.source.dataset['choiceSource'] = '';
			this.source.innerHTML = this.innerHTML;
			this.replaceChildren(this.source);
		}
		const declarations = [...this.source.content.children];
		if (
			!declarations.length ||
			declarations.some((child): boolean => child.localName !== 'ex-option')
		)
			throw new Error('A choice needs direct ex-option children.');
		const { renderChoiceTitle, renderNarrationContent } =
			await import('../render/markdown-content.ts');
		const fieldset = document.createElement('fieldset');
		const legend = document.createElement('legend');
		legend.textContent = this.question;
		fieldset.append(legend);
		const tableView = this.getAttribute('view') === 'table';
		this.dataset['width'] = this.getAttribute('data-width') ?? (tableView ? 'canvas' : 'reading');
		const table = document.createElement('table');
		const tableBody = table.createTBody();
		if (tableView) {
			const head = table.createTHead().insertRow();
			for (const title of ['Choose', 'Option', 'Summary', 'Details', 'Comments']) {
				const cell = document.createElement('th');
				cell.scope = 'col';
				cell.textContent = title;
				head.append(cell);
			}
			const scroll = document.createElement('div');
			scroll.className = 'ex-table-scroll ex-choice-table';
			scroll.append(table);
			fieldset.append(scroll);
		}
		const ids = new Set<string>();
		const rendered: RenderedOption[] = [];
		for (const [index, declaration] of declarations.entries()) {
			const optionId = declaration.id.trim();
			const title = declaration.getAttribute('title')?.trim();
			const summarySource = declaration.querySelector(':scope > template[slot="summary"]');
			if (declaration.querySelectorAll(':scope > template[slot="summary"]').length !== 1)
				throw new Error('Each option needs exactly one summary template.');
			if (
				!optionId ||
				ids.has(optionId) ||
				optionId === this.questionId ||
				!title ||
				!(summarySource instanceof HTMLTemplateElement) ||
				!summarySource.content.textContent?.trim()
			)
				throw new Error(
					'Each option needs a unique authored id, inline title and nonempty summary template.',
				);
			ids.add(optionId);
			if (!this.copiedResponse && this.ownerDocument.getElementById(optionId))
				throw new Error(`Duplicate option id: ${optionId}`);
			const target = document.createElement(tableView ? 'tr' : 'ex-option');
			target.id = this.copiedResponse ? `${this.instanceId}-option-${index}` : optionId;
			target.className = 'ex-choice-option';
			const input = document.createElement('input');
			input.type = this.hasAttribute('multiple') ? 'checkbox' : 'radio';
			input.name = this.instanceId;
			input.value = optionId;
			input.id = `${this.instanceId}-input-${index}`;
			input.disabled = this.copiedResponse !== undefined;
			input.checked =
				this.copiedResponse?.selected.some((option): boolean => option.optionId === optionId) ??
				false;
			const label = document.createElement('label');
			label.htmlFor = input.id;
			label.className = 'ex-choice-label';
			label.innerHTML = renderChoiceTitle(title);
			const optionLabel = label.textContent?.trim() ?? title;
			target.dataset['commentLabel'] = optionLabel;
			const summary = document.createElement('div');
			summary.className = 'ex-choice-summary';
			summary.id = `${this.instanceId}-summary-${index}`;
			summary.innerHTML = renderNarrationContent(
				summarySource.content.textContent ?? '',
				this.baseURI,
			);
			input.setAttribute('aria-describedby', summary.id);
			const titleRow = document.createElement('div');
			titleRow.className = 'ex-choice-title-row';
			titleRow.append(label);
			const detailsSource = declaration.querySelector<HTMLElement>(':scope > [slot="details"]');
			const details =
				detailsSource && this.copiedResponse
					? copyInspectionContent(detailsSource)
					: detailsSource?.cloneNode(true);
			let depth: HTMLElement | undefined;
			if (details instanceof HTMLElement) {
				if (this.getAttribute('details') === 'inline') {
					depth = document.createElement('details');
					depth.className = 'ex-choice-inline';
					const toggle = document.createElement('summary');
					toggle.textContent = 'Details';
					details.removeAttribute('slot');
					depth.append(toggle, details);
				} else {
					depth = document.createElement('div');
					depth.className = 'ex-choice-depth';
					depth.setAttribute('label', optionLabel);
					details.hidden = true;
					depth.append(details);
					const owner = depth;
					const button = createIconButton('article', `Open details for ${optionLabel}`, (): void =>
						openIsland(owner, 'sidebar'),
					);
					button.classList.add('ex-choice-details-button');
					button.append('Details');
					titleRow.append(button);
				}
			}
			if (target instanceof HTMLTableRowElement) {
				target.insertCell().append(input);
				const heading = document.createElement('th');
				heading.scope = 'row';
				heading.append(titleRow);
				target.append(heading);
				target.insertCell().append(summary);
				if (depth) target.insertCell().append(depth);
				else target.insertCell();
				target.insertCell().dataset['commentSlot'] = '';
				tableBody.append(target);
			} else {
				const body = document.createElement('div');
				body.className = 'ex-choice-copy';
				body.append(titleRow, summary);
				if (depth) body.append(depth);
				target.append(input, body);
				fieldset.append(target);
			}
			rendered.push({ optionId, label: optionLabel, input, target });
		}
		this.options = rendered;
		this.replaceChildren(this.source, fieldset);
		this.initialized = true;
		this.normalizeInlineHeadings();
		if (this.isConnected) this.connectControls();
		registerInspectionCopy(this, (): HTMLElement => this.inspectionCopy());
		this.setAttribute('label', this.question);
		installIslandInspector(this);
		this.dispatchEvent(new Event('island-ready', { bubbles: true }));
	}

	private normalizeInlineHeadings(): void {
		for (const heading of this.querySelectorAll('.ex-choice-inline :is(h1,h2,h3,h4,h5,h6)')) {
			const local = document.createElement('div');
			local.className = 'ex-choice-local-heading';
			local.setAttribute('role', 'heading');
			local.setAttribute('aria-level', heading.localName.slice(1));
			if (heading.id) local.id = heading.id;
			local.append(...heading.childNodes);
			heading.replaceWith(local);
		}
	}

	private connectControls(): void {
		this.events.abort();
		this.events = new AbortController();
		const { signal } = this.events;
		this.addEventListener('island-ready', (): void => this.normalizeInlineHeadings(), { signal });
		if (this.copiedResponse) return;
		for (const option of this.options)
			option.target.addEventListener(
				'click',
				(event: MouseEvent): void => {
					if (
						event
							.composedPath()
							.some(
								(node): boolean =>
									node instanceof Element &&
									node.matches(
										'a,button,input,label,textarea,select,summary,[contenteditable]:not([contenteditable="false"]),[data-comment-control]',
									),
							) ||
						window.getSelection()?.toString()
					)
						return;
					option.input.click();
				},
				{ signal },
			);
		this.addEventListener(
			'change',
			(event: Event): void => {
				if (!this.options.some((option): boolean => option.input === event.target)) return;
				this.dispatchEvent(
					new CustomEvent<ChoiceResponse>('choice-change', {
						bubbles: true,
						composed: true,
						detail: this.response,
					}),
				);
			},
			{ signal },
		);
	}

	private inspectionCopy(): HTMLElement {
		const copy = document.createElement('ex-choice');
		if (!(copy instanceof ChoiceIsland) || !this.source)
			throw new Error('Choice is not registered.');
		copy.id = this.questionId;
		copy.setAttribute('title', this.question);
		for (const attribute of ['multiple', 'view', 'details', 'data-width']) {
			const value = this.getAttribute(attribute);
			if (value !== null) copy.setAttribute(attribute, value);
		}
		copy.copiedResponse = this.response;
		copy.append(this.source.cloneNode(true));
		return copy;
	}
}

export function readChoiceResponses(root: HTMLElement): ChoiceResponse[] {
	return [...root.querySelectorAll('ex-choice')]
		.filter(
			(element): element is ChoiceIsland =>
				element instanceof ChoiceIsland &&
				!element.closest('[data-inspector-copy],[data-inspector-content]'),
		)
		.map((element): ChoiceResponse => element.response)
		.filter((response): boolean => response.selected.length > 0);
}
