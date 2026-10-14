import { reportIssue } from '../core/issues.ts';
import { normalizeIslandText } from '../core/source-text.ts';
import { createToneIcon, readTone } from '../core/tones.ts';
import { type CodeLine, parseCode } from '../render/code-lines.ts';
import { installIslandInspector } from './inspector.ts';

/** A margin note attached to one line of an ex-code island. */
export class CodeNoteIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) return;
		const tone = readTone(this, 'note');
		if (tone) this.prepend(createToneIcon(tone));
		this.setAttribute('prepared', '');
	}
}

function changeCounts(lines: readonly CodeLine[]): HTMLElement {
	const counts = document.createElement('span');
	counts.className = 'ex-code-counts';
	const added = lines.filter((line): boolean => line.kind === 'added').length;
	const removed = lines.filter((line): boolean => line.kind === 'removed').length;
	counts.innerHTML = `<b data-kind="added">+${added}</b> <b data-kind="removed">−${removed}</b>`;
	return counts;
}

/**
 * Line-numbered code or a unified diff (one file or many), with ex-note children shown beside the
 * lines they name. In a multi-file diff a note names its file: <ex-note file="resume.ts" line="43">.
 */
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
		const files = parseCode(lines, diff, firstNumber);
		const multiple = files.length > 1;
		const notes = Array.from(this.querySelectorAll<HTMLElement>(':scope > ex-note'));
		const placed = new Set<HTMLElement>();
		const header = document.createElement('div');
		header.className = 'ex-code-head';
		const title = this.getAttribute('title') ?? (multiple ? undefined : files[0]?.path);
		if (title) {
			this.dataset['title'] = title;
			this.removeAttribute('title');
			this.setAttribute('label', this.getAttribute('label') ?? title);
			const name = document.createElement('strong');
			name.textContent = title;
			header.append(name);
		}
		if (multiple) {
			const summary = document.createElement('strong');
			summary.textContent = `${files.length} files`;
			if (!title) header.append(summary);
		}
		const language = this.getAttribute('lang');
		if (language) {
			const chip = document.createElement('span');
			chip.textContent = language;
			header.append(chip);
		}
		if (diff) header.append(changeCounts(files.flatMap((file): CodeLine[] => file.lines)));
		const sections = files.map((file): HTMLElement => {
			const body = document.createElement('div');
			body.className = 'ex-code-lines';
			const fileNotes = notes.filter((note): boolean => {
				const named = note.getAttribute('file');
				return !multiple || (named !== null && file.path?.endsWith(named) === true);
			});
			for (const line of file.lines) {
				const number = document.createElement('span');
				number.className = 'ex-code-number';
				const code = document.createElement('code');
				code.className = 'ex-code-text';
				code.dataset['kind'] = line.kind;
				code.textContent = line.text || ' ';
				const margin = document.createElement('span');
				margin.className = 'ex-code-margin';
				if (line.number !== undefined) {
					number.textContent = String(line.number);
					const lineNotes = fileNotes.filter(
						(note): boolean =>
							!placed.has(note) &&
							Number.parseInt(note.getAttribute('line') ?? '', 10) === line.number,
					);
					if (lineNotes.length > 0) {
						for (const note of lineNotes) placed.add(note);
						margin.append(...lineNotes);
						const tone = lineNotes[0]?.getAttribute('tone') ?? 'note';
						number.setAttribute('tone', tone);
						number.dataset['noted'] = '';
						if (line.kind === 'context') {
							code.setAttribute('tone', tone);
							code.dataset['noted'] = '';
						}
						margin.dataset['noted'] = '';
					}
				}
				body.append(number, code, margin);
			}
			if (!multiple) return body;
			const section = document.createElement('details');
			section.className = 'ex-code-file';
			section.open = true;
			const name = document.createElement('summary');
			const path = document.createElement('span');
			path.textContent = file.path ?? 'file';
			name.append(path, changeCounts(file.lines));
			section.append(name, body);
			return section;
		});
		for (const note of notes)
			if (!placed.has(note))
				reportIssue(
					note,
					multiple && !note.hasAttribute('file')
						? `This diff has ${files.length} files; add file="…" to say which one line="${note.getAttribute('line') ?? ''}" is in.`
						: `line="${note.getAttribute('line') ?? ''}"${note.hasAttribute('file') ? ` in file="${note.getAttribute('file') ?? ''}"` : ''} is not a line of this code.`,
				);
		this.replaceChildren(...(header.childElementCount > 0 ? [header] : []), ...sections);
		this.setAttribute('prepared', '');
	}
}
