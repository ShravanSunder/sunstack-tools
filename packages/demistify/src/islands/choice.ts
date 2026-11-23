import { decisionDetailsId } from '../core/design-record-view.ts';
import { createIconButton } from '../core/icons.ts';
import { reportIssue } from '../core/issues.ts';
import { type ChoiceResponse, choiceResponseSchema } from '../core/review-model.ts';
import { checkAttributes } from '../vocabulary.ts';
import { ChoiceDetailPanel } from './choice-detail-panel.ts';
import { appendRecordBasis, choiceRecordFaces, recordStateLine } from './design-record-face.ts';
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
	private decisionId: string | undefined;
	private question = '';
	private rendering: Promise<void> | undefined;
	private events = new AbortController();
	private copiedResponse: ChoiceResponse | undefined;
	private initialized = false;

	get response(): ChoiceResponse {
		return choiceResponseSchema.parse({
			questionId: this.questionId,
			...(this.decisionId && { decisionId: this.decisionId }),
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
		const face = choiceRecordFaces.get(this);
		const bound = face?.record.kind === 'choice' ? face.record : undefined;
		const settled = bound && bound.authority.state !== 'owner-open' ? bound.authority : undefined;
		const explanation = this.querySelector<HTMLTemplateElement>(
			':scope > template[data-record-body]',
		);
		explanation?.remove();
		this.questionId = this.copiedResponse?.questionId ?? this.id.trim();
		this.decisionId = this.copiedResponse?.decisionId ?? this.getAttribute('data-decision')?.trim();
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
		const detailsMode = bound
			? 'beside'
			: (this.getAttribute('details') ?? (tableView ? 'sidebar' : 'beside'));
		const hasDetails = declarations.some((option): boolean =>
			Boolean(option.querySelector(':scope > [slot="details"]')),
		);
		const beside =
			detailsMode === 'beside' && !tableView && (hasDetails || bound)
				? new ChoiceDetailPanel()
				: undefined;
		const authoredWidth = this.getAttribute('data-width');
		this.dataset['width'] = authoredWidth ?? (tableView || beside ? 'canvas' : 'reading');
		this.classList.toggle('ex-choice-default-width', authoredWidth === null);
		this.classList.toggle('ex-choice-cards', !tableView);
		if (!tableView) {
			legend.addEventListener('click', (event): void => {
				if (!(event instanceof PointerEvent) || event.pointerType !== 'touch') return;
				legend.tabIndex = -1;
				legend.focus({ preventScroll: true });
			});
		}
		if (beside) {
			fieldset.classList.add('ex-choice-beside-fieldset');
			if (explanation) fieldset.append(explanation.content.cloneNode(true));
			fieldset.append(beside.layout);
		}
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
			const recordOption = declaration.getAttribute('data-option');
			const chosen = Boolean(settled && settled.chosen === recordOption);
			if (settled) target.classList.add(chosen ? 'ex-record-chosen' : 'ex-record-alternative');
			const input = document.createElement('input');
			input.type = this.hasAttribute('multiple') ? 'checkbox' : 'radio';
			input.name = this.instanceId;
			input.value = optionId;
			input.id = `${this.instanceId}-input-${index}`;
			input.disabled = Boolean(settled) || this.copiedResponse !== undefined;
			input.hidden = Boolean(settled);
			input.checked =
				this.copiedResponse?.selected.some((option): boolean => option.optionId === optionId) ??
				chosen;
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
			const optionTitle = document.createElement('span');
			optionTitle.append(label);
			if (
				settled ||
				(bound?.recommended !== 'none' && bound?.recommended?.option === recordOption)
			) {
				const mark = document.createElement('span');
				mark.className = chosen ? 'ex-card-pick' : 'ex-record-option-mark';
				mark.textContent = settled
					? chosen
						? 'Chosen'
						: 'Not chosen'
					: `Recommended · ${bound?.recommended !== 'none' ? bound?.recommended?.by.alias : ''}`;
				optionTitle.append(mark);
			}
			titleRow.append(bound ? optionTitle : label);
			const authoredDetails = declaration.querySelector<HTMLElement>(':scope > [slot="details"]');
			const detailsSource = authoredDetails ?? (bound ? document.createElement('div') : undefined);
			if (detailsSource && !authoredDetails)
				detailsSource.textContent = 'No additional details are recorded.';
			const details =
				detailsSource && this.copiedResponse
					? copyInspectionContent(detailsSource)
					: detailsSource?.cloneNode(true);
			let depth: HTMLElement | undefined;
			if (details instanceof HTMLElement) {
				if (detailsMode === 'inline') {
					depth = document.createElement('details');
					depth.className = 'ex-choice-inline';
					const toggle = document.createElement('summary');
					toggle.textContent = 'Details';
					details.removeAttribute('slot');
					depth.append(toggle, details);
				} else if (beside) {
					let panelId =
						bound && recordOption
							? decisionDetailsId(this.questionId, recordOption)
							: `${target.id}--choice-details`;
					while (
						this.ownerDocument.getElementById(panelId) ||
						this.source.content.getElementById(panelId)
					) {
						panelId += '-panel';
					}
					if (bound) {
						const back = document.createElement('a');
						back.href = '#' + encodeURIComponent(this.questionId);
						back.textContent = 'Back to the decision';
						details.append(back);
					}
					const button = beside.add(
						details,
						bound ? `${this.question} · ${optionLabel}` : optionLabel,
						panelId,
					);
					button.classList.add('ex-choice-details-button');
					if (bound) button.classList.add('ex-record-detail-button');
					else button.append('Details');
					titleRow.append(button);
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
				(beside?.options ?? fieldset).append(target);
			}
			rendered.push({ optionId, label: optionLabel, input, target });
		}
		this.options = rendered;
		this.replaceChildren(this.source, fieldset);
		if (face && beside) {
			this.prepend(recordStateLine(face, this.questionId, beside));
			appendRecordBasis(this, face);
			this.removeAttribute('title');
			this.classList.add('ex-bound-choice');
		}
		this.initialized = true;
		this.normalizeDetailHeadings();
		if (this.isConnected) this.connectControls();
		registerInspectionCopy(this, (): HTMLElement => this.inspectionCopy());
		this.setAttribute('label', this.question);
		if (!bound) installIslandInspector(this);
		this.dispatchEvent(new Event('island-ready', { bubbles: true }));
	}

	private normalizeDetailHeadings(): void {
		for (const heading of this.querySelectorAll(
			':is(.ex-choice-inline, .ex-choice-detail-panel) :is(h1,h2,h3,h4,h5,h6)',
		)) {
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
		this.addEventListener('island-ready', (): void => this.normalizeDetailHeadings(), { signal });
		if (
			this.copiedResponse ||
			(choiceRecordFaces.get(this)?.record.kind === 'choice' &&
				this.options.every(({ input }) => input.disabled))
		)
			return;
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
					if (
						option.target.localName === 'ex-option' &&
						event instanceof PointerEvent &&
						event.pointerType === 'touch'
					) {
						option.input.focus({ preventScroll: true });
						return;
					}
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
			if (attribute === 'data-width' && this.classList.contains('ex-choice-default-width'))
				continue;
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
