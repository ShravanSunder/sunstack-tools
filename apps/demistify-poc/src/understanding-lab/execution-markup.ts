import { requirementHref } from './study-links.js';
import { icon, safeText } from './study-markup.js';
import { type FrameExample } from './study-model.js';

/** Categories are authored roles, using the same palette slots as the color study. */
export function frameColor(category: FrameExample['category']): string {
	return category === 'Plumbing'
		? 'var(--ink-soft)'
		: `var(--chart-${category === 'Input' ? 1 : category === 'Action' ? 2 : 3})`;
}

function callTitleMarkup(name: string): string {
	const argumentStart = name.indexOf('(');
	const callName = argumentStart < 0 ? name : name.slice(0, argumentStart);
	const argumentsText = argumentStart < 0 ? '' : name.slice(argumentStart);
	return `${safeText(callName).replaceAll('.', '.<wbr>')}<wbr><span class="lab-frame-arguments">${safeText(argumentsText)}</span>`;
}

function frameButton(frame: FrameExample, index: number): string {
	const title = callTitleMarkup(frame.name);
	return `<button type="button" data-frame="${index}" style="--frame-color:${frameColor(frame.category)}" aria-pressed="false"><span class="lab-call-index">${index + 1}</span><span class="lab-frame-copy"><span class="lab-frame-title"><strong>${title}</strong>${icon('caret-right')}</span><span class="lab-frame-description">${safeText(frame.responsibility)}</span><span class="lab-frame-meta"><span>${safeText(frame.category)}</span><small>${safeText(frame.source)}</small></span></span></button>`;
}

export function callStackMarkup(frames: readonly FrameExample[], expanded: boolean): string {
	const rows: string[] = [];
	let depth = 0;
	for (let index = 0; index < frames.length; index++) {
		const frame = frames[index];
		if (!frame) continue;
		if (frame.supporting) {
			const hidden: string[] = [];
			let supporting = frames[index];
			while (supporting?.supporting) {
				hidden.push(
					`<div class="lab-call-card" data-call-index="${index}" data-comment-label="${safeText(supporting.name)}" style="--frame-color:${frameColor(supporting.category)}">${frameButton(supporting, index)}<span class="lab-call-comments" data-comment-slot></span></div>`,
				);
				index++;
				supporting = frames[index];
			}
			index--;
			rows.push(
				`<li style="--frame-depth:${depth}"><lab-hidden-calls ${expanded ? 'expanded' : ''}>${hidden.join('')}</lab-hidden-calls></li>`,
			);
		} else {
			rows.push(
				`<li class="lab-call-card" data-call-index="${index}" data-comment-label="${safeText(frame.name)}" style="--frame-depth:${depth};--frame-color:${frameColor(frame.category)}">${frameButton(frame, index)}<span class="lab-call-comments" data-comment-slot></span>${frame.branch ? `<div class="lab-branch-note">${icon('git-branch')}<span>${safeText(frame.branch)}</span></div>` : ''}</li>`,
			);
		}
		depth++;
	}
	return `<ol class="lab-call-stack">${rows.join('')}</ol>`;
}

export function frameDetailMarkup(frame: FrameExample, preview: boolean): string {
	return `<small>${preview ? 'Preview' : 'Selected call'} · ${safeText(frame.category)}</small><div class="lab-local-heading" role="heading" aria-level="3">${callTitleMarkup(frame.name)}</div><p class="lab-detail-description">${safeText(frame.responsibility)}</p><div class="lab-source-caption">${icon('code')}<span>${safeText(frame.source)}</span></div><pre><code>${safeText(frame.statement)}</code></pre><div class="lab-frame-requirement">${frame.requirement ? `<a class="lab-navigation-action" href="${requirementHref(frame.requirement)}">${icon('squares-four')} Requirements · ${frame.requirement}</a>` : '<p>Supporting plumbing. No direct requirement link is declared for this call.</p>'}</div>${frame.branch ? `<p class="lab-frame-branch">${icon('git-branch')} ${safeText(frame.branch)}</p>` : ''}`;
}
