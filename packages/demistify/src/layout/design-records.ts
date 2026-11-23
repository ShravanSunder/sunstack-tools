import {
	designRecordsFilename,
	designRecordViewSchema,
	type DesignRecordView,
} from '../core/design-record-view.ts';
import { reportIssue } from '../core/issues.ts';

const placementSelector = '[data-design-record], ex-tree[data-design-need]';

function recordsSource(): URL | undefined {
	const implicit =
		(
			[
				['ex-nav[src]', 'navigation.generated.json'],
				['ex-project-overview[src]', 'project.generated.json'],
			] as const
		)
			.map(([selector, filename]): string[] =>
				Array.from(document.querySelectorAll(selector)).flatMap((element): string[] => {
					const source = element.getAttribute('src') ?? '';
					return source.split('/').at(-1) === filename ? [source] : [];
				}),
			)
			.find((sources): boolean => sources.length > 0) ?? [];
	const markers = document.querySelectorAll<HTMLMetaElement>('meta[name="demistify-records"]');
	if (implicit.length > 1 || (implicit.length === 0 && markers.length !== 1)) return undefined;
	const [binding] = implicit;
	const source = binding ?? markers[0]?.content.trim();
	if (!source || /^(?:[a-z][a-z0-9+.-]*:|\/|\\)/iu.test(source)) return undefined;
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
		if (anchor?.hasAttribute('data-design-record'))
			requestAnimationFrame((): void => anchor.scrollIntoView());
	} catch {
		// A malformed URL fragment has no target.
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
			const record = placements.get(element.getAttribute('data-design-record') ?? '');
			if (record) {
				element.id = record.reference;
				if (record.decision !== undefined) {
					element.setAttribute('data-decision', record.decision);
					element.title = record.statement;
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
		message.textContent =
			'Design records are unavailable. Run demistify project check --write (or demistify design check --write for a single milestone), then reload.';
		(document.querySelector('ex-document') ?? document.body).prepend(message);
	}
}

export function hydrateDesignRecords(): Promise<void> | undefined {
	const elements = Array.from(document.querySelectorAll<HTMLElement>(placementSelector));
	if (elements.length) return fillBindings(elements);
	return undefined;
}
