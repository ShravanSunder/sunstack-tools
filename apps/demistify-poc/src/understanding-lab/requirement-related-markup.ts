import { readCollection, type PreparedRecord } from './collection-data.js';
import { localDocumentBody, recordsForNeed, scenarioPresentation } from './requirement-stages.js';
import { icon, safeText } from './study-markup.js';
import type { RequirementExample } from './study-model.js';

function recordTitle(item: PreparedRecord): string {
	return 'title' in item.record ? item.record.title : item.record.id;
}

function sourceLink(item: PreparedRecord): string {
	return `<a href="#related-record-${safeText(item.record.id)}">${safeText(item.record.id)} · ${safeText(recordTitle(item))}</a>`;
}

function ruleBody(item: PreparedRecord): string {
	return (
		[...item.bodyHtml.matchAll(/<p(?:\s[^>]*)?>([\s\S]*?)<\/p>/g)].at(-1)?.[1] ??
		safeText(item.summary)
	);
}

/**
 * Private HTML authoring hooks: the first `figure.lab-related-visual` is the primary sketch;
 * `.lab-document-gaps` and `.lab-specification-cases` provide compact related-view content.
 * `.lab-proof-design` is split by scenarioBodyParts. Full bodies stay with their record cards.
 */
function recordFragment(item: PreparedRecord, selector: string): string {
	const template = document.createElement('template');
	template.innerHTML = item.bodyHtml;
	const fragment = template.content.querySelector(selector);
	return fragment
		? localDocumentBody(fragment.outerHTML, 'related-preview-' + item.record.id + '-')
		: '';
}

function scenarioMarkup(item: PreparedRecord): string {
	return `<section class="lab-related-record" id="related-record-${safeText(item.record.id)}" data-record-id="${safeText(item.record.id)}" tabindex="-1"><div class="lab-related-record-title">${sourceLink(item)}</div>${scenarioPresentation(item, 'related-preview-' + item.record.id + '-')}</section>`;
}

function otherDesignNeedLinks(item: PreparedRecord, currentNeed: string): string {
	if (item.record.kind !== 'program-design') return '';
	const collection = readCollection();
	const specificationIds = item.record.specifications;
	const needs = new Set(
		collection.records.flatMap(({ record }) =>
			record.kind === 'specification' && specificationIds.includes(record.id) ? record.needs : [],
		),
	);
	const otherNeeds = [...needs].filter((id) => id !== currentNeed);
	return otherNeeds.length
		? `<p class="lab-record-relations">Also serves ${otherNeeds.map((id) => `<button type="button" class="lab-related-need" data-need="${safeText(id)}">${safeText(id)}</button>`).join(', ')}</p>`
		: '';
}

function designMarkup(items: readonly PreparedRecord[], currentNeed: string): string {
	return items
		.map((item) => {
			return `<section class="lab-related-record"><div class="lab-related-record-title">${sourceLink(item)}</div><p>${safeText(item.summary)}</p>${recordFragment(item, 'figure.lab-related-visual')}${recordFragment(item, '.lab-document-gaps')}${otherDesignNeedLinks(item, currentNeed)}${sourceDetails(item)}</section>`;
		})
		.join('');
}

function sourceDetails(item: PreparedRecord): string {
	return `<details class="lab-record-source"><summary>Source details · ${safeText(item.record.id)}</summary><section id="related-record-${safeText(item.record.id)}" tabindex="-1">${localDocumentBody(item.bodyHtml, `related-body-${item.record.id}-`)}</section></details>`;
}

/** One selected requirement's linked records, projected inside its existing controller. */
export function relatedRequirementMarkup(need: RequirementExample): string {
	const collection = readCollection();
	const specifications = recordsForNeed(collection, need.id, 'specification');
	const designs = recordsForNeed(collection, need.id, 'program-design');
	const scenarios = recordsForNeed(collection, need.id, 'verification');
	const specification = `<section class="lab-expanded-specification"><div class="lab-column-heading" role="heading" aria-level="3">Specification</div>${specifications.map((item) => `<section class="lab-related-record lab-rule-layout"><div><div class="lab-related-record-title">${sourceLink(item)}</div><p>${ruleBody(item)}</p></div>${recordFragment(item, '.lab-specification-cases')}${sourceDetails(item)}</section>`).join('')}</section>`;
	const design = `<section class="lab-expanded-design"><div class="lab-column-heading"><span role="heading" aria-level="3">Program Design</span><button type="button" class="lab-navigation-action lab-icon-action" data-action="open-calls" aria-label="Open call stack" title="Call stack">${icon('git-branch')}</button></div>${designMarkup(designs, need.id)}</section>`;
	const proof = `<section class="lab-expanded-proof"><div class="lab-column-heading" role="heading" aria-level="3">Verification</div>${scenarios.map(scenarioMarkup).join('')}<p class="lab-soft">Illustrative fixture results. A target build, setup and real-path check are still needed for product proof.</p></section>`;
	return `<div class="lab-requirement-expanded"><p class="lab-related-need-statement">${safeText(need.need)}</p><div class="lab-expanded-grid">${specification}${design}${proof}</div></div>`;
}
