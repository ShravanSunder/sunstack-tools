import type { RequirementExample } from './collection-model.js';
import {
	readCollection,
	type PreparedCollection,
	type PreparedRecord,
	type ScenarioOutcome,
} from './collection.ts';
import type { RequirementStage } from './document-links.ts';
import { icon, outcomeBadge, safeText } from './document-markup.ts';

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
export function scenarioOutcomes(
	collection: PreparedCollection,
	scenarioId: string,
): readonly ScenarioOutcome[] {
	const outcomes = collection.records.flatMap(({ record }): ScenarioOutcome[] =>
		record.kind === 'observation' && record.scenario === scenarioId ? [record.outcome] : [],
	);
	return outcomes.length ? outcomes : ['Not run'];
}
/** Keep document section headings local to the component rather than the page outline. */
export function localDocumentBody(html: string, idPrefix = ''): string {
	const template = document.createElement('template');
	template.innerHTML = html;
	for (const heading of template.content.querySelectorAll('h1,h2,h3,h4,h5,h6')) {
		const local = document.createElement('div');
		for (const attribute of heading.attributes) local.setAttribute(attribute.name, attribute.value);
		local.classList.add('ex-design-record-label');
		local.setAttribute('role', 'heading');
		local.setAttribute('aria-level', heading.tagName.slice(1));
		local.append(...heading.childNodes);
		heading.replaceWith(local);
	}
	if (idPrefix) {
		const ids = new Map(
			[...template.content.querySelectorAll('[id]')].map((element) => [
				element.id,
				`${idPrefix}${element.id}`,
			]),
		);
		for (const element of template.content.querySelectorAll('*')) {
			const replacement = ids.get(element.id);
			if (replacement) element.id = replacement;
			for (const name of [
				'for',
				'aria-labelledby',
				'aria-describedby',
				'aria-controls',
				'aria-owns',
				'aria-details',
				'aria-activedescendant',
				'headers',
			]) {
				const value = element.getAttribute(name);
				if (value)
					element.setAttribute(
						name,
						value
							.split(/\s+/)
							.map((id) => ids.get(id) ?? id)
							.join(' '),
					);
			}
			const href = element.getAttribute('href');
			if (href?.startsWith('#') && ids.has(href.slice(1)))
				element.setAttribute('href', `#${ids.get(href.slice(1))}`);
		}
	}
	return template.innerHTML;
}
/** Explicit procedure content stays separate from supplied observations. */
export function scenarioBodyParts(item: PreparedRecord): {
	scenarioHtml: string;
	proposalHtml: string;
} {
	const template = document.createElement('template');
	template.innerHTML = item.bodyHtml;
	const proposal = template.content.querySelector('[data-ex-part="procedure"]');
	const proposalHtml = proposal?.outerHTML ?? '';
	proposal?.remove();
	return { scenarioHtml: template.innerHTML, proposalHtml };
}

/** The stage and related reader show the same scenario and supplied observations. */
export function scenarioPresentation(item: PreparedRecord, idPrefix = ''): string {
	const observations = readCollection().records.filter(
		({ record }) => record.kind === 'observation' && record.scenario === item.record.id,
	);
	const { scenarioHtml, proposalHtml } = scenarioBodyParts(item);
	const outcome = observations.length
		? observations
				.map(
					(observation) =>
						`<section class="ex-design-example-outcome" data-record-id="${observation.record.id}"><div class="ex-design-stage-record-heading"><strong>Result · ${safeText(observation.record.id)}</strong>${observation.record.kind === 'observation' ? outcomeBadge(observation.record.outcome) : ''}</div>${localDocumentBody(observation.bodyHtml, `${idPrefix}${observation.record.id}-`)}</section>`,
				)
				.join('')
		: `<div class="ex-design-example-outcome"><div class="ex-design-stage-record-heading"><strong>Result</strong>${outcomeBadge('Not run')}</div><p>No outcome is attached to this scenario.</p></div>`;
	return `${localDocumentBody(scenarioHtml, idPrefix)}${outcome}${localDocumentBody(proposalHtml, idPrefix)}`;
}
export function scenarioBadges(collection: PreparedCollection, needId: string): string {
	return `<span class="ex-design-scenario-badges">${recordsForNeed(
		collection,
		needId,
		'verification',
	)
		.flatMap(({ record }) =>
			scenarioOutcomes(collection, record.id).map((outcome) =>
				outcomeBadge(outcome, `${'title' in record ? record.title : record.id}: ${outcome}`),
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
		return `<div class="ex-design-stage-summaries">${stages
			.map(
				(stage) =>
					`<section id="${stage.id}-${need.id}"><div class="ex-design-stage-summary-heading"><div class="ex-design-local-heading" role="heading" aria-level="4">${stage.title}</div><button type="button" class="ex-design-navigation-action" data-document="${stage.id}" aria-label="${stage.title} details">Details ${icon('arrow-right')}</button></div>${
						recordsForNeed(collection, need.id, stage.id)
							.map(
								(item) =>
									`<div class="ex-design-summary-record" data-record-id="${item.record.id}"><div><span class="ex-design-soft">${safeText(item.record.id)}</span> · <strong>${safeText('title' in item.record ? item.record.title : '')}</strong>${
										stage.id === 'verification'
											? scenarioOutcomes(collection, item.record.id)
													.map((outcome) => outcomeBadge(outcome))
													.join('')
											: ''
									}</div><p>${safeText(item.summary)}</p></div>`,
							)
							.join('') || '<p role="status">Not yet written.</p>'
					}</section>`,
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
						? `<button type="button" class="ex-design-related-need" data-document="specification" data-context-need="${record.needs[0]}" data-record-target="${record.id}">${record.id}</button>`
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
							`<button type="button" class="ex-design-related-need" data-document="${stage}" data-record-target="${record.id}">${record.id}</button>`,
					)
					.join(', ');
			const designLinks =
				focused.id !== 'program-design' ? relatedRecordLinks('program-design') : '';
			const checkLinks = focused.id !== 'verification' ? relatedRecordLinks('verification') : '';
			const body =
				focused.id === 'verification'
					? scenarioPresentation(item)
					: localDocumentBody(item.bodyHtml);
			return `<section class="ex-design-stage-record" data-record-id="${item.record.id}"><div class="ex-design-stage-record-heading"><div class="ex-design-local-heading" role="heading" aria-level="4" id="record-${item.record.id}"><span class="ex-design-soft">${item.record.id}</span> · ${safeText('title' in item.record ? item.record.title : item.summary)}</div></div>${body}<footer class="ex-design-record-links">${specifications.length ? `<p class="ex-design-record-relations">Specification: ${specificationLinks}</p>` : ''}${designLinks ? `<p class="ex-design-record-relations">Related design: ${designLinks}</p>` : ''}${checkLinks ? `<p class="ex-design-record-relations">Related checks: ${checkLinks}</p>` : ''}${backlinks.length ? `<p class="ex-design-record-relations">Also serves ${backlinks.map((id) => `<button type="button" class="ex-design-related-need" data-need="${id}">${id}</button>`).join(', ')}</p>` : ''}</footer></section>`;
		})
		.join('');
	return `<section class="ex-design-stage-detail" id="${focused.id}-${need.id}">${bodies || '<p role="status">Not yet written.</p>'}</section>`;
}
