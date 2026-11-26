import {
	designRecordsFilename,
	designRecordViewSchema,
	migratedOptionAnchor,
	recordSourceHref,
	type DesignRecordView,
} from '../core/design-record-view.ts';
import { reportIssue } from '../core/issues.ts';
import {
	choiceRecordFaces,
	inlineRecordStatus,
	renderSectionRecord,
} from '../islands/design-record-face.ts';

const placementSelector =
	'[data-design-record], [data-design-status], ex-section-item[record], ex-tree[data-design-need]';

function recordsSource(): URL | undefined {
	let implicit: string[] = [];
	for (const [selector, filename] of [
		['ex-nav[src]', 'navigation.generated.json'],
		['ex-initiative-overview[src]', 'initiative.generated.json'],
	] as const) {
		implicit = Array.from(document.querySelectorAll(selector)).flatMap((element): string[] => {
			const source = recordSourceHref(element.getAttribute('src') ?? '', filename);
			return source === undefined ? [] : [source];
		});
		if (implicit.length) break;
	}
	const markers = document.querySelectorAll<HTMLMetaElement>('meta[name="demistify-records"]');
	if (implicit.length > 1 || (implicit.length === 0 && markers.length !== 1)) return undefined;
	const [binding] = implicit;
	const marker = markers[0]?.content ?? '';
	const source =
		binding ?? (/[?#]/u.test(marker) ? undefined : recordSourceHref(marker, designRecordsFilename));
	if (!source) return undefined;
	const current = new URL(document.URL);
	const address = new URL(source, current);
	if (
		address.host !== current.host ||
		address.origin !== current.origin ||
		address.protocol !== current.protocol
	)
		return undefined;
	const script = binding ? new URL(designRecordsFilename, address) : address;
	if (
		!['file:', 'http:', 'https:'].includes(script.protocol) ||
		script.protocol !== current.protocol ||
		script.origin !== current.origin ||
		script.host !== current.host ||
		script.pathname.split('/').at(-1) !== designRecordsFilename ||
		script.search ||
		script.hash
	)
		return undefined;
	return script;
}

function loadRecords(source: URL): Promise<DesignRecordView> {
	return new Promise((resolve, reject): void => {
		const script = document.createElement('script');
		script.src = source.href;
		script.addEventListener(
			'load',
			(): void => {
				const serialized = script.dataset['demistifyRecords'];
				script.remove();
				try {
					resolve(designRecordViewSchema.parse(JSON.parse(serialized ?? 'null')));
				} catch (error) {
					reject(error instanceof Error ? error : new Error(String(error)));
				}
			},
			{ once: true },
		);
		script.addEventListener(
			'error',
			(): void => {
				script.remove();
				reject(new Error('Cannot load generated records.'));
			},
			{ once: true },
		);
		document.head.append(script);
	});
}

function pagePath(address: URL): string {
	return decodeURIComponent(
		address.pathname.endsWith('/') ? new URL('index.html', address).pathname : address.pathname,
	);
}

function restoreFragment(): void {
	const hash = location.hash.slice(1);
	if (!hash) return;
	try {
		const anchor = document.getElementById(decodeURIComponent(hash));
		if (
			anchor?.matches(
				'[data-design-record], ex-section-item[record], ex-section-item[record] ex-choice',
			)
		)
			requestAnimationFrame((): void => anchor.scrollIntoView());
	} catch {
		// A malformed URL fragment has no target.
	}
}

type GeneratedPlacement = DesignRecordView['pages'][string]['placements'][number];

function hydrateChoice(choice: HTMLElement, placement: GeneratedPlacement): void {
	if (!placement.decision) return;
	choice.id = placement.reference;
	choice.setAttribute('data-decision', placement.decision);
	choice.title = placement.statement;
	// Rolling v1 pages generated before record faces already supply their own options.
	if (placement.face?.record.kind !== 'choice') return;
	choiceRecordFaces.set(choice, placement.face);
	const authored = [...choice.querySelectorAll(':scope > ex-option[data-option]')];
	for (const option of placement.face.record.options) {
		const declaration = authored.find((item) => item.getAttribute('data-option') === option.id);
		if (!declaration) continue;
		declaration.id =
			declaration.getAttribute(migratedOptionAnchor) ?? `${placement.reference}--${option.id}`;
		declaration.setAttribute('title', option.title);
		choice.append(declaration);
	}
}

async function fillBindings(elements: readonly HTMLElement[]): Promise<void> {
	try {
		const source = recordsSource();
		if (!source) throw new Error('No local generated records source.');
		const records = await loadRecords(source);
		const current = pagePath(new URL(document.URL));
		const match = Object.entries(records.pages).find(
			([href]): boolean => pagePath(new URL(href, source)) === current,
		);
		if (!match) throw new Error('No generated records for this page.');
		const [, page] = match;
		const placements = new Map(page.placements.map((record) => [record.reference, record]));
		const trees = new Map(page.trees.map((tree) => [tree.reference, tree.markdown]));
		for (const element of elements) {
			if (element.matches('ex-section-item[record]')) {
				const reference = element.getAttribute('record');
				const item = page.sectionItems?.find((entry) => entry.reference === reference);
				const placement = reference ? placements.get(reference) : undefined;
				if (!item || !placement?.face)
					throw new Error(`No checked section record for ${reference}.`);
				const choice = element.querySelector<HTMLElement>(':scope > ex-choice');
				if (Boolean(choice) !== (placement.face.record.kind === 'choice'))
					throw new Error(
						`The checked section record ${reference} does not match its authored item.`,
					);
				element.dataset['sectionHold'] = item.holds;
				continue;
			}
			const status = element.getAttribute('data-design-status');
			if (status !== null) {
				if (!page.statuses?.some(({ reference }) => reference === status))
					throw new Error(`No generated status for ${status}.`);
				continue;
			}
			const placement = element.getAttribute('data-design-record');
			if (placement !== null) {
				const record = placements.get(placement);
				if (!record || element.matches('ex-choice') !== (record.decision !== undefined))
					throw new Error(`No generated placement for ${placement}.`);
			} else {
				const reference = element.getAttribute('data-design-need') ?? '';
				if (!trees.has(reference)) throw new Error(`No generated Tree for ${reference}.`);
			}
		}
		for (const element of elements) {
			if (element.matches('ex-section-item[record]')) {
				const placement = placements.get(element.getAttribute('record') ?? '');
				if (!placement?.face) continue;
				const choice = element.querySelector<HTMLElement>(':scope > ex-choice');
				if (choice) {
					hydrateChoice(choice, placement);
				} else {
					element.id = placement.reference;
					renderSectionRecord(element, placement.face);
				}
				continue;
			}
			const status = page.statuses?.find(
				({ reference }) => reference === element.getAttribute('data-design-status'),
			);
			if (status) {
				inlineRecordStatus(element, status.record, status.href);
				continue;
			}
			const record = placements.get(element.getAttribute('data-design-record') ?? '');
			if (record) {
				element.id = record.reference;
				if (record.decision !== undefined) {
					hydrateChoice(element, record);
				} else {
					element.textContent = record.statement;
				}
			} else {
				const template = document.createElement('template');
				template.content.textContent =
					trees.get(element.getAttribute('data-design-need') ?? '') ?? '';
				element.replaceChildren(template);
			}
		}
		restoreFragment();
	} catch (error) {
		reportIssue(undefined, error instanceof Error ? error.message : String(error));
		const message = document.createElement('p');
		message.className = 'ex-design-records-status';
		message.setAttribute('role', 'status');
		for (const section of document.querySelectorAll<HTMLElement>('ex-section[holds]'))
			section.hidden = true;
		message.textContent = document.querySelector('ex-project-overview')
			? 'Design records are unavailable. ex-project-overview was replaced by ex-initiative-overview in 1.34. Update the page, regenerate its design records, and reload.'
			: 'Design records are unavailable. Run demistify initiative check --write (or demistify design check --write for a standalone project), then reload.';
		const scope = document.querySelector('ex-document');
		if (scope) scope.before(message);
		else document.body.prepend(message);
	}
}

export function hydrateDesignRecords(): Promise<void> | undefined {
	const elements = Array.from(document.querySelectorAll<HTMLElement>(placementSelector));
	if (elements.length) return fillBindings(elements);
	return undefined;
}
