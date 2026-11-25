import { dirname, relative, resolve, sep } from 'node:path';

/**
 * How references and links change when records, deliverables and page sections move.
 * A reference is written relative to the project that holds it: bare inside its own
 * project, qualified (`sessions/R1`) from anywhere else.
 */
export interface AddressMaps {
	/** Old qualified record address to new qualified address, for every moved record. */
	readonly records: ReadonlyMap<string, string>;
	/** Old qualified deliverable address to new qualified address. */
	readonly deliverables: ReadonlyMap<string, string>;
}

export type AddressKind = 'record' | 'deliverable';

/** The project a reference is written in, before and after the run (none on a root page). */
export interface ReferenceOwner {
	readonly before: string | undefined;
	readonly after: string | undefined;
}

export function rewriteReference(
	reference: string,
	kind: AddressKind,
	owner: ReferenceOwner,
	maps: AddressMaps,
): string {
	const slash = reference.indexOf('/');
	if (slash === -1 && owner.before === undefined) return reference;
	const qualified = slash === -1 ? `${owner.before}/${reference}` : reference;
	const moved = (kind === 'record' ? maps.records : maps.deliverables).get(qualified) ?? qualified;
	return owner.after !== undefined && moved.startsWith(`${owner.after}/`)
		? moved.slice(owner.after.length + 1)
		: moved;
}

export interface AnchorTarget {
	readonly file: string;
	readonly fragment: string;
}

const externalHref = /^(?:[a-z][a-z0-9+.-]*:|\/)/iu;

function decode(value: string): string {
	try {
		return decodeURIComponent(value);
	} catch {
		return value;
	}
}

function encodePath(path: string): string {
	return path
		.split(sep)
		.map((segment) => encodeURIComponent(segment))
		.join('/');
}

/**
 * Moves of HTML anchors: a placement whose reference changed, or an id carried with
 * a moved section. A derived anchor (`R14--details`) follows its placement.
 */
export class AnchorRelocations {
	readonly #moves = new Map<string, Map<string, AnchorTarget>>();
	readonly #prefixes = new Map<string, Map<string, AnchorTarget>>();

	move(file: string, fragment: string, target: AnchorTarget): void {
		const page = this.#moves.get(file) ?? new Map<string, AnchorTarget>();
		page.set(fragment, target);
		this.#moves.set(file, page);
	}

	/** A placement anchor and every `${fragment}--…` anchor derived from it. */
	movePlacement(file: string, fragment: string, target: AnchorTarget): void {
		this.move(file, fragment, target);
		const page = this.#prefixes.get(file) ?? new Map<string, AnchorTarget>();
		page.set(fragment, target);
		this.#prefixes.set(file, page);
	}

	target(file: string, fragment: string): AnchorTarget {
		const direct = this.#moves.get(file)?.get(fragment);
		if (direct) return direct;
		for (const [prefix, target] of this.#prefixes.get(file) ?? []) {
			if (fragment.startsWith(`${prefix}--`))
				return {
					file: target.file,
					fragment: `${target.fragment}${fragment.slice(prefix.length)}`,
				};
		}
		return { file, fragment };
	}

	/**
	 * Rewrite a relative link written in `fromBefore` so it still reaches the same
	 * passage when read from `fromAfter`. Unchanged links keep their exact text.
	 */
	relocate(href: string, fromBefore: string, fromAfter: string): string {
		if (externalHref.test(href) || href.startsWith('//')) return href;
		const hashAt = href.indexOf('#');
		const pathAndQuery = hashAt === -1 ? href : href.slice(0, hashAt);
		const fragment = hashAt === -1 ? undefined : decode(href.slice(hashAt + 1));
		const queryAt = pathAndQuery.indexOf('?');
		const path = queryAt === -1 ? pathAndQuery : pathAndQuery.slice(0, queryAt);
		const query = queryAt === -1 ? '' : pathAndQuery.slice(queryAt);
		const file = path ? resolve(dirname(fromBefore), decode(path)) : fromBefore;
		const target =
			fragment === undefined ? { file, fragment: undefined } : this.target(file, fragment);
		const sameTarget = target.file === file && target.fragment === fragment;
		const samePlace = path ? dirname(fromBefore) === dirname(fromAfter) : fromBefore === fromAfter;
		if (sameTarget && samePlace) return href;
		const fragmentText =
			target.fragment === undefined
				? ''
				: `#${encodeURIComponent(target.fragment).replace(/%2F/gu, '/')}`;
		if (target.file === fromAfter && !query && fragmentText) return fragmentText;
		const relativePath = relative(dirname(fromAfter), target.file);
		return `${encodePath(relativePath || '.')}${query}${fragmentText}`;
	}
}
