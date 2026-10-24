import {
	commentSnapshotSchema,
	createCommentSnapshot,
	documentIdentitySchema,
	pageCommentSchema,
	type CommentAnchor,
	type CommentSnapshot,
	type DocumentIdentity,
} from './review-model.ts';

export type PersistenceState =
	| { readonly kind: 'ready' }
	| { readonly kind: 'unavailable'; readonly message: string }
	| { readonly kind: 'invalid'; readonly message: string };
export interface CommentStorage {
	readonly getItem: (key: string) => string | null;
	readonly setItem: (key: string, value: string) => void;
}
export interface CommentStore {
	readonly snapshot: () => CommentSnapshot;
	readonly persistence: () => PersistenceState;
	readonly save: (anchor: CommentAnchor, body: string, commentId?: string) => void;
}

export function commentStorageKey(identity: DocumentIdentity): string {
	return `demistify-review-1:${JSON.stringify([identity.documentId, identity.documentRevision])}`;
}

/** Invalid saved bytes stay untouched; fresh feedback remains available in memory. */
export function createCommentStore(
	identity: DocumentIdentity,
	storage: () => CommentStorage = (): Storage => localStorage,
): CommentStore {
	const validatedIdentity = documentIdentitySchema.parse(identity);
	const storageKey = commentStorageKey(validatedIdentity);
	let snapshot = createCommentSnapshot(validatedIdentity);
	let persistence: PersistenceState = { kind: 'ready' };
	let stored: string | null = null;
	try {
		stored = storage().getItem(storageKey);
	} catch {
		persistence = {
			kind: 'unavailable',
			message: 'Local saving is unavailable. Copy or export to keep your review.',
		};
	}
	if (stored !== null) {
		try {
			const restored = commentSnapshotSchema.parse(JSON.parse(stored));
			if (
				restored.documentId !== validatedIdentity.documentId ||
				restored.documentRevision !== validatedIdentity.documentRevision
			)
				throw new Error('Saved identity differs.');
			snapshot = restored;
		} catch {
			persistence = {
				kind: 'invalid',
				message:
					'Saved comments are invalid and have been preserved. New comments are unsaved; copy or export to keep them.',
			};
		}
	}
	return {
		snapshot: (): CommentSnapshot => structuredClone(snapshot),
		persistence: (): PersistenceState => ({ ...persistence }),
		save(anchor, body, commentId): void {
			const previous =
				commentId === undefined
					? undefined
					: snapshot.comments.find((comment): boolean => comment.commentId === commentId);
			if (commentId !== undefined && !previous)
				throw new Error('The comment to edit is unavailable.');
			const now = new Date().toISOString();
			const comment = pageCommentSchema.parse({
				commentId: previous?.commentId ?? crypto.randomUUID(),
				anchor: previous?.anchor ?? anchor,
				body,
				createdAt: previous?.createdAt ?? now,
				updatedAt: now,
			});
			snapshot = {
				...snapshot,
				comments: previous
					? snapshot.comments.map((item): typeof comment =>
							item.commentId === comment.commentId ? comment : item,
						)
					: [...snapshot.comments, comment],
			};
			if (persistence.kind === 'invalid') return;
			try {
				storage().setItem(storageKey, JSON.stringify(snapshot));
				persistence = { kind: 'ready' };
			} catch {
				persistence = {
					kind: 'unavailable',
					message: 'Could not save locally. Copy or export to keep your review.',
				};
			}
		},
	};
}
