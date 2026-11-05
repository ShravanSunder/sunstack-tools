import { readCollection, type PreparedCollection, type PreparedRecord } from './collection-data.ts';
import type { RequirementStage } from './study-links.ts';
import { icon, outcomeBadge, safeText } from './study-markup.ts';
import type { RequirementExample } from './study-model.js';

/** Backlinks are derived from the one authored forward edge at each owner. */
export function recordsForNeed(
	collection: PreparedCollection,
	needId: string,
	stage: RequirementStage,
): readonly PreparedRecord[] {
	const specifications = collection.records.filter(
		({ record }) => record.kind === 'specification' && record.needs.includes(needId),
	);
	if (stage === 'specification') return specifications;
	const ids = new Set(specifications.map(({ record }) => record.id));
	return collection.records.filter(
		({ record }) =>
			record.kind === (stage === 'program-design' ? 'program-design' : 'scenario') &&
			'specifications' in record &&
			record.specifications.some((id) => ids.has(id)),
	);
}
export function scenarioOutcome(
	collection: PreparedCollection,
	scenarioId: string,
): RequirementExample['outcome'] {
	const observation = collection.records.find(
		({ record }) => record.kind === 'observation' && record.scenario === scenarioId,
	);
	return observation?.record.kind === 'observation' ? observation.record.outcome : 'Not run';
}
/** Keep document section headings local to the component rather than the page outline. */
export function localDocumentBody(html: string, idPrefix = ''): string {
	const template = document.createElement('template');
	template.innerHTML = html;
	for (const heading of template.content.querySelectorAll('h1,h2,h3,h4,h5,h6')) {
		const local = document.createElement('div');
		for (const attribute of heading.attributes) local.setAttribute(attribute.name, attribute.value);
		local.classList.add('lab-record-label');
		local.setAttribute('role', 'heading');
		local.setAttribute('aria-level', heading.tagName.slice(1));
		local.append(...heading.childNodes);
		heading.replaceWith(local);
	}
	if (idPrefix)
		for (const element of template.content.querySelectorAll('[id]'))
			element.id = `${idPrefix}${element.id}`;
	return template.innerHTML;
}
/** `.lab-proof-design` is a private body-authoring hook, separated from example outcomes. */
export function scenarioBodyParts(item: PreparedRecord): {
	scenarioHtml: string;
	proposalHtml: string;
} {
	const template = document.createElement('template');
	template.innerHTML = item.bodyHtml;
	const proposal = template.content.querySelector('.lab-proof-design');
	const proposalHtml = proposal?.outerHTML ?? '';
	proposal?.remove();
	return { scenarioHtml: template.innerHTML, proposalHtml };
}

/** The tab and the combined reader show the same scenario and example evidence. */
export function scenarioPresentation(item: PreparedRecord, idPrefix = ''): string {
	const observations = readCollection().records.filter(
		({ record }) => record.kind === 'observation' && record.scenario === item.record.id,
	);
	const { scenarioHtml, proposalHtml } = scenarioBodyParts(item);
	const outcome = observations.length
		? observations
				.map(
					(observation) =>
						`<section class="lab-example-outcome" data-record-id="${observation.record.id}"><div class="lab-stage-record-heading"><strong>Illustrative result</strong>${observation.record.kind === 'observation' ? outcomeBadge(observation.record.outcome) : ''}</div>${localDocumentBody(observation.bodyHtml, idPrefix)}<small>${observation.record.id} · Example only · target not supplied</small></section>`,
				)
				.join('')
		: `<div class="lab-example-outcome"><div class="lab-stage-record-heading"><strong>Illustrative result</strong>${outcomeBadge('Not run')}</div><p>No example outcome is attached to this scenario.</p></div>`;
	return `${localDocumentBody(scenarioHtml, idPrefix)}${outcome}${localDocumentBody(proposalHtml, idPrefix)}`;
}
export function scenarioBadges(collection: PreparedCollection, needId: string): string {
	return `<span class="lab-scenario-badges">${recordsForNeed(collection, needId, 'verification')
		.map(({ record }) =>
			outcomeBadge(
				scenarioOutcome(collection, record.id),
				`${'title' in record ? record.title : record.id}: ${scenarioOutcome(collection, record.id)}`,
			),
		)
		.join('')}</span>`;
}
export function stageMarkup(
	need: RequirementExample,
	focusedStage: RequirementStage | undefined,
): string {
	const collection = readCollection();
	const stages = [
		{ id: 'specification', title: 'Specification', icon: 'file-text' },
		{ id: 'program-design', title: 'Program Design', icon: 'tree-structure' },
		{ id: 'verification', title: 'Verification', icon: 'check-circle' },
	] satisfies { id: RequirementStage; title: string; icon: string }[];
	const focused = stages.find((stage) => stage.id === focusedStage);
	if (!focused)
		return `<div class="lab-stage-summaries">${stages
			.map(
				(stage) =>
					`<section id="${stage.id}-${need.id}"><div class="lab-stage-summary-heading"><div class="lab-local-heading" role="heading" aria-level="4">${stage.title}</div><button type="button" class="lab-navigation-action" data-document="${stage.id}" aria-label="${stage.title} details">Details ${icon('arrow-right')}</button></div>${recordsForNeed(
						collection,
						need.id,
						stage.id,
					)
						.map(
							(item) =>
								`<div class="lab-summary-record" data-record-id="${item.record.id}"><div><span class="lab-soft">${safeText(item.record.id)}</span> · <strong>${safeText('title' in item.record ? item.record.title : '')}</strong>${stage.id === 'verification' ? outcomeBadge(scenarioOutcome(collection, item.record.id)) : ''}</div><p>${safeText(item.summary)}</p></div>`,
						)
						.join('')}</section>`,
			)
			.join('')}</div>`;
	const bodies = recordsForNeed(collection, need.id, focused.id)
		.map((item) => {
			const specifications =
				item.record.kind === 'program-design' || item.record.kind === 'scenario'
					? item.record.specifications
					: [];
			const relatedSpecifications = collection.records.filter(
				({ record }) => record.kind === 'specification' && specifications.includes(record.id),
			);
			const backlinks =
				item.record.kind === 'specification'
					? item.record.needs.filter((id) => id !== need.id)
					: [
							...new Set(
								relatedSpecifications.flatMap(({ record }) =>
									record.kind === 'specification' ? record.needs : [],
								),
							),
						].filter((id) => id !== need.id);
			const specificationLinks = relatedSpecifications
				.map(({ record }) =>
					record.kind === 'specification'
						? `<button type="button" class="lab-related-need" data-document="specification" data-context-need="${record.needs[0]}" data-record-target="${record.id}">${record.id}</button>`
						: '',
				)
				.join(', ');
			const selectedSpecifications = new Set(
				recordsForNeed(collection, need.id, 'specification').map(({ record }) => record.id),
			);
			const contextSpecifications = new Set(
				item.record.kind === 'specification'
					? [item.record.id]
					: specifications.filter((id) => selectedSpecifications.has(id)),
			);
			const relatedRecordLinks = (stage: RequirementStage): string =>
				recordsForNeed(collection, need.id, stage)
					.filter(
						({ record }) =>
							(record.kind === 'program-design' || record.kind === 'scenario') &&
							record.id !== item.record.id &&
							record.specifications.some((id) => contextSpecifications.has(id)),
					)
					.map(
						({ record }) =>
							`<button type="button" class="lab-related-need" data-document="${stage}" data-record-target="${record.id}">${record.id}</button>`,
					)
					.join(', ');
			const designLinks =
				focused.id !== 'program-design' ? relatedRecordLinks('program-design') : '';
			const checkLinks = focused.id !== 'verification' ? relatedRecordLinks('verification') : '';
			const body =
				focused.id === 'verification'
					? scenarioPresentation(item)
					: localDocumentBody(item.bodyHtml);
			return `<section class="lab-stage-record" data-record-id="${item.record.id}"><div class="lab-stage-record-heading"><div class="lab-local-heading" role="heading" aria-level="4" id="record-${item.record.id}"><span class="lab-soft">${item.record.id}</span> · ${safeText('title' in item.record ? item.record.title : item.summary)}</div></div>${body}<footer class="lab-record-links">${specifications.length ? `<p class="lab-record-relations">Specification: ${specificationLinks}</p>` : ''}${designLinks ? `<p class="lab-record-relations">Related design: ${designLinks}</p>` : ''}${checkLinks ? `<p class="lab-record-relations">Related checks: ${checkLinks}</p>` : ''}${backlinks.length ? `<p class="lab-record-relations">Also serves ${backlinks.map((id) => `<button type="button" class="lab-related-need" data-need="${id}">${id}</button>`).join(', ')}</p>` : ''}</footer></section>`;
		})
		.join('');
	return `<section class="lab-stage-detail" id="${focused.id}-${need.id}">${bodies}</section>`;
}
