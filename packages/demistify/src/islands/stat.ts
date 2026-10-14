import { readTone } from '../core/tones.ts';

/** A single number with its label and a short note; the tone colours the note. */
export class StatIsland extends HTMLElement {
	connectedCallback(): void {
		if (this.hasAttribute('prepared')) return;
		readTone(this, undefined);
		const value = document.createElement('strong');
		value.className = 'ex-stat-value';
		value.textContent = this.getAttribute('value') ?? '';
		const label = document.createElement('span');
		label.className = 'ex-stat-label';
		label.textContent = this.getAttribute('label') ?? this.textContent.trim();
		const parts: HTMLElement[] = [value, label];
		const noteText = this.getAttribute('note');
		if (noteText) {
			const note = document.createElement('span');
			note.className = 'ex-stat-note';
			note.textContent = noteText;
			parts.push(note);
		}
		this.replaceChildren(...parts);
		this.setAttribute('prepared', '');
	}
}
