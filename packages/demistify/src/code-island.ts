import { installIslandInspector } from './island-inspector.ts';
import { reportIssue } from './issues.ts';
import { normalizeIslandText } from './source-text.ts';
import { createToneIcon, readTone } from './tones.ts';

type LineKind = 'added' | 'removed' | 'context' | 'hunk';

function lineKind(line: string, diff: boolean): LineKind {
	if (!diff) return 'context';
	if (line.startsWith('@@')) return 'hunk';
	if (line.startsWith('+')) return 'added';
	if (line.startsWith('-')) return 'removed';
	return 'context';
}

/** A margin note attached to one line of an ex-code island. */
export class CodeNoteIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) return;
		const tone = readTone(this, 'note');
		if (tone) this.prepend(createToneIcon(tone));
		this.setAttribute('prepared', '');
	}
}

/** Line-numbered code or a unified diff, with ex-note children shown beside the lines they name. */
export class CodeIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('data-inspector-content')) return;
		if (!this.hasAttribute('prepared')) this.renderCode();
		installIslandInspector(this);
	}

	private renderCode(): void {
		const wrapper = this.querySelector(':scope > script, :scope > template');
		const sourceText = normalizeIslandText(
			wrapper instanceof HTMLTemplateElement
				? (wrapper.content.textContent ?? '')
				: (wrapper?.textContent ?? ''),
		);
		const lines = sourceText.split('\n');
		const diff =
			this.getAttribute('lang') === 'diff' || lines.some((line): boolean => line.startsWith('@@'));
		const firstNumber = Number.parseInt(this.getAttribute('start') ?? '1', 10) || 1;
		const notes = new Map<number, HTMLElement[]>();
		for (const note of this.querySelectorAll<HTMLElement>(':scope > ex-note')) {
			const line = Number.parseInt(note.getAttribute('line') ?? '', 10);
			notes.set(line, [...(notes.get(line) ?? []), note]);
		}
		const header = document.createElement('div');
		header.className = 'ex-code-head';
		const title = this.getAttribute('title');
		if (title) {
			this.dataset['title'] = title;
			this.removeAttribute('title');
			this.setAttribute('label', this.getAttribute('label') ?? title);
			const name = document.createElement('strong');
			name.textContent = title;
			header.append(name);
		}
		const language = this.getAttribute('lang');
		if (language) {
			const chip = document.createElement('span');
			chip.textContent = language;
			header.append(chip);
		}
		const body = document.createElement('div');
		body.className = 'ex-code-lines';
		let lineNumber = firstNumber;
		const placed = new Set<number>();
		for (const line of lines) {
			const kind = lineKind(line, diff);
			const number = document.createElement('span');
			number.className = 'ex-code-number';
			const code = document.createElement('code');
			code.className = 'ex-code-text';
			code.dataset['kind'] = kind;
			code.textContent = line || ' ';
			const margin = document.createElement('span');
			margin.className = 'ex-code-margin';
			if (kind !== 'hunk') {
				number.textContent = String(lineNumber);
				const lineNotes = notes.get(lineNumber);
				if (lineNotes) {
					placed.add(lineNumber);
					margin.append(...lineNotes);
					const tone = lineNotes[0]?.getAttribute('tone') ?? 'note';
					number.setAttribute('tone', tone);
					number.dataset['noted'] = '';
					if (kind === 'context') {
						code.setAttribute('tone', tone);
						code.dataset['noted'] = '';
					}
					margin.dataset['noted'] = '';
				}
				lineNumber += 1;
			}
			body.append(number, code, margin);
		}
		for (const [line, lineNotes] of notes)
			if (!placed.has(line))
				for (const note of lineNotes)
					reportIssue(
						note,
						`line="${note.getAttribute('line') ?? ''}" is not a line of this code (lines ${firstNumber}–${lineNumber - 1}).`,
					);
		this.replaceChildren(...(header.childElementCount > 0 ? [header] : []), body);
		this.setAttribute('prepared', '');
	}
}
