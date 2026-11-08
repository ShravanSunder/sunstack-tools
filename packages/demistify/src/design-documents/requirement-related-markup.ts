import type { RequirementExample } from './collection-model.js';
import { readCollection, type PreparedRecord } from './collection.js';
import { icon, safeText } from './document-markup.js';
import {
	localDocumentBody,
	recordsForNeed,
	scenarioPresentation,
	needBodyMarkup,
} from './requirement-stages.js';

function recordTitle(item: PreparedRecord): string {
	return 'title' in item.record ? item.record.title : item.record.id;
}

function sourceLink(item: PreparedRecord): string {
	return `<a href="#related-record-${safeText(item.record.id)}">${safeText(item.record.id)} · ${safeText(recordTitle(item))}</a>`;
}

function recordFragment(item: PreparedRecord, selector: string): string {
	const template = document.createElement('template');
	template.innerHTML = item.bodyHtml;
	const fragment = template.content.querySelector(selector);
	return fragment
		? localDocumentBody(fragment.outerHTML, 'related-preview-' + item.record.id + '-')
		: '';
}

function scenarioMarkup(item: PreparedRecord): string {
	return `<section class="ex-design-related-record" id="related-record-${safeText(item.record.id)}" data-record-id="${safeText(item.record.id)}" tabindex="-1"><div class="ex-design-related-record-title">${sourceLink(item)}</div>${scenarioPresentation(item, 'related-preview-' + item.record.id + '-')}</section>`;
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
		? `<p class="ex-design-record-relations">Also serves ${otherNeeds.map((id) => `<button type="button" class="ex-design-related-need" data-need="${safeText(id)}">${safeText(id)}</button>`).join(', ')}</p>`
		: '';
}

function designMarkup(items: readonly PreparedRecord[], currentNeed: string): string {
	return items
		.map((item) => {
			return `<section class="ex-design-related-record"><div class="ex-design-related-record-title">${sourceLink(item)}</div><p>${safeText(item.summary)}</p>${recordFragment(item, '[data-ex-part="sketch"]')}${recordFragment(item, '[data-ex-part="gaps"]')}${otherDesignNeedLinks(item, currentNeed)}${sourceDetails(item)}</section>`;
		})
		.join('');
}

function sourceDetails(item: PreparedRecord): string {
	return `<details class="ex-design-record-source"><summary>Source details · ${safeText(item.record.id)}</summary><section id="related-record-${safeText(item.record.id)}" tabindex="-1">${localDocumentBody(item.bodyHtml, `related-body-${item.record.id}-`)}</section></details>`;
}

/** One selected requirement's linked records, projected inside its existing controller. */
export function relatedRequirementMarkup(need: RequirementExample): string {
	const collection = readCollection();
	const specifications = recordsForNeed(collection, need.id, 'specification');
	const designs = recordsForNeed(collection, need.id, 'program-design');
	const scenarios = recordsForNeed(collection, need.id, 'verification');
	const specification = `<section class="ex-design-expanded-specification"><div class="ex-design-column-heading" role="heading" aria-level="3">Specification</div>${specifications.map((item) => `<section class="ex-design-related-record ex-design-rule-layout"><div><div class="ex-design-related-record-title">${sourceLink(item)}</div>${recordFragment(item, '[data-ex-part="rule"]')}</div>${recordFragment(item, '[data-ex-part="cases"]')}${sourceDetails(item)}</section>`).join('') || '<p role="status">Not yet written.</p>'}</section>`;
	const design = `<section class="ex-design-expanded-design"><div class="ex-design-column-heading"><span role="heading" aria-level="3">Program Design</span><button type="button" class="ex-design-navigation-action ex-design-icon-action" data-action="open-calls" aria-label="Open call stack" title="Call stack">${icon('git-branch')}</button></div>${designMarkup(designs, need.id) || '<p role="status">Not yet written.</p>'}</section>`;
	const proof = `<section class="ex-design-expanded-proof"><div class="ex-design-column-heading" role="heading" aria-level="3">Verification</div>${scenarios.map(scenarioMarkup).join('') || '<p role="status">Not yet written.</p>'}</section>`;
	return `<div class="ex-design-requirement-expanded">${needBodyMarkup(need, 'related')}<div class="ex-design-expanded-grid">${specification}${design}${proof}</div></div>`;
}
