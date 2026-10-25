import { documentIdentitySchema, type DocumentIdentity } from './review-model.ts';

export type ReviewSourceResult =
	| { readonly kind: 'loaded'; readonly source: string }
	| { readonly kind: 'failed' };
interface ReviewDocumentSource {
	readonly markup: string;
	readonly inputs: readonly HTMLElement[];
	revision?: Promise<string>;
}
const documentSources = new WeakMap<HTMLElement, ReviewDocumentSource>();
const sourceLoads = new WeakMap<HTMLElement, Promise<ReviewSourceResult>>();
const inspectionCopy = '[data-inspector-copy],[data-inspector-content]';

/** Capture source before upgrades replace it with rendered views and transient controls. */
export function captureReviewDocument(root: HTMLElement): void {
	if (root.closest(inspectionCopy) || documentSources.has(root)) return;
	const markup = root.outerHTML;
	if (!root.id) {
		let index = 1;
		while (root.ownerDocument.getElementById(`ex-content${index === 1 ? '' : `-${index}`}`))
			index++;
		root.id = `ex-content${index === 1 ? '' : `-${index}`}`;
	}
	const inputs = [
		...root.querySelectorAll<HTMLElement>('ex-markdown[src]:not([prepared]),ex-explorer[src]'),
	].filter((element): boolean => !element.closest(inspectionCopy));
	documentSources.set(root, { markup, inputs });
}

/** Register the complete existing load synchronously, before its first import/await settles. */
export function registerReviewSource(owner: HTMLElement, load: Promise<ReviewSourceResult>): void {
	if (!owner.closest(inspectionCopy) && !sourceLoads.has(owner)) sourceLoads.set(owner, load);
}

async function fingerprintReviewSource(source: ReviewDocumentSource): Promise<string> {
	// All initial custom-element connectedCallbacks have registered by this microtask.
	await Promise.resolve();
	const inputs = await Promise.all(
		source.inputs.map((owner): Promise<ReviewSourceResult> => {
			const load = sourceLoads.get(owner);
			if (!load) throw new Error(`Comments source load was not registered: ${owner.localName}.`);
			return load;
		}),
	);
	const bytes = new TextEncoder().encode(JSON.stringify([source.markup, inputs]));
	const digest = await crypto.subtle.digest('SHA-256', bytes);
	return `sha256:${[...new Uint8Array(digest)].map((byte): string => byte.toString(16).padStart(2, '0')).join('')}`;
}

export async function resolveReviewDocumentIdentity(root: HTMLElement): Promise<DocumentIdentity> {
	captureReviewDocument(root);
	const source = documentSources.get(root);
	if (!source) throw new Error('Comments needs an original document.');
	if (!root.hasAttribute('revision')) {
		source.revision ??= fingerprintReviewSource(source);
		const revision = await source.revision;
		if (!root.hasAttribute('revision')) root.setAttribute('revision', revision);
	}
	if (!root.hasAttribute('document-id')) {
		const address = new URL(root.ownerDocument.URL);
		address.username = '';
		address.password = '';
		address.search = '';
		address.hash = '';
		root.setAttribute('document-id', address.href);
	}
	return documentIdentitySchema.parse({
		documentId: root.getAttribute('document-id'),
		documentRevision: root.getAttribute('revision'),
	});
}

export function installDefaultComments(): void {
	if (
		[...document.querySelectorAll('ex-comments')].some(
			(element): boolean => !element.closest(inspectionCopy),
		)
	)
		return;
	const root = [...document.querySelectorAll<HTMLElement>('ex-document')].find(
		(element): boolean => !element.closest(inspectionCopy),
	);
	if (!root || root.getAttribute('comments') === 'off') return;
	captureReviewDocument(root);
	const comments = document.createElement('ex-comments');
	comments.setAttribute('for', root.id);
	document.body.append(comments);
}
