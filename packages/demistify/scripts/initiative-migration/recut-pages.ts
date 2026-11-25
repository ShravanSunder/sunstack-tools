import { escapeHtml } from '../../src/core/html-text.ts';
import type { SectionHold } from '../../src/core/section-routing.ts';
import {
	AnchorRelocations,
	rewriteReference,
	type AddressMaps,
	type AnchorTarget,
} from './address-rewrite.ts';
import {
	headingSection,
	itemUnit,
	parsePage,
	type Binding,
	type PageSource,
	type PageUnit,
	type ParsedPage,
} from './page-units.ts';
import { localId, projectOf } from './recut-resolution.ts';
import { applyTextEdits, type TextEdit } from './source-edits.ts';

export type { PageSource, PageUnit } from './page-units.ts';

export interface MovedPageUnit extends PageUnit {
	readonly target: string;
	readonly targetProject: string;
	readonly records: readonly string[];
	readonly deliverables: readonly string[];
}

export interface UnplacedItem {
	/** The item's qualified address after the run. */
	readonly address: string;
	readonly file: string;
	readonly reason: string;
}

export interface PageMoveRequest {
	/** Old qualified address of each moved record, with the project it goes to. */
	readonly records: ReadonlyMap<
		string,
		{ readonly to: string; readonly holds: readonly SectionHold[] }
	>;
	/** Old qualified address of each moved deliverable, its target project and its destination. */
	readonly deliverables: ReadonlyMap<
		string,
		{ readonly to: string; readonly newAddress: string; readonly destination: AnchorTarget }
	>;
	/** Destination of every deliverable that stays, so a section holding it is not taken. */
	readonly stayingDestinations: readonly AnchorTarget[];
	/** The page each project receives moved passages on. */
	readonly targetPages: ReadonlyMap<string, string>;
	/** Pages each project owns: only these give up passages. */
	readonly ownedPages: ReadonlyMap<string, string>;
	/** How a reason names a file. */
	readonly label: (file: string) => string;
}

export interface PagePlan {
	readonly files: ReadonlyMap<string, string>;
	readonly anchors: AnchorRelocations;
	readonly moved: readonly MovedPageUnit[];
	readonly unplaced: readonly UnplacedItem[];
	/** Where each moved deliverable's destination is after the run. */
	readonly destinations: ReadonlyMap<string, AnchorTarget>;
	/** The page each moved record is placed on after the run. */
	readonly recordPlacements: ReadonlyMap<string, string>;
}

function unitKey(unit: PageUnit): string {
	return `${unit.page}:${unit.start}:${unit.end}`;
}

function qualified(owner: string | undefined, reference: string): string | undefined {
	if (reference.includes('/')) return reference;
	return owner === undefined ? undefined : `${owner}/${reference}`;
}

function within(unit: { start: number; end: number }, position: number): boolean {
	return position >= unit.start && position < unit.end;
}

function attributeEdit(name: string, value: string, start: number, end: number): TextEdit {
	return { start, end, text: `${name}="${escapeHtml(value)}"` };
}

interface CandidateUnit extends PageUnit {
	readonly targetProject: string;
	readonly records: Set<string>;
	readonly deliverables: Set<string>;
	readonly holds: Set<SectionHold>;
}

export function planPages(
	sources: readonly PageSource[],
	request: PageMoveRequest,
	maps: AddressMaps,
): PagePlan {
	const pages = new Map(sources.map((source) => [source.file, parsePage(source)]));
	const unplaced: UnplacedItem[] = [];
	const candidates = new Map<string, CandidateUnit>();
	const addCandidate = (
		unit: PageUnit,
		targetProject: string,
		item: { record?: string; deliverable?: string; holds?: readonly SectionHold[] },
	): void => {
		const key = unitKey(unit);
		const candidate =
			candidates.get(key) ??
			({
				...unit,
				targetProject,
				records: new Set(),
				deliverables: new Set(),
				holds: new Set(),
			} satisfies CandidateUnit);
		if (item.record) candidate.records.add(item.record);
		if (item.deliverable) candidate.deliverables.add(item.deliverable);
		for (const hold of item.holds ?? []) candidate.holds.add(hold);
		candidates.set(key, candidate);
	};
	const blocked = new Map<string, string>();
	const placementFiles = new Map<string, string>();

	for (const page of pages.values()) {
		for (const binding of page.bindings) {
			if (binding.kind !== 'placement') continue;
			const address = qualified(page.owner, binding.reference);
			const move = address === undefined ? undefined : request.records.get(address);
			if (!address || !move) continue;
			placementFiles.set(address, page.file);
			const source = projectOf(address);
			if (request.ownedPages.get(page.file) !== source) {
				blocked.set(
					address,
					`placed on ${request.label(page.file)}, a page its project does not own`,
				);
				continue;
			}
			const unit =
				binding.element.tagName === 'ex-section-item'
					? itemUnit(page, binding.element)
					: headingSection(page, binding.element);
			if (!unit) {
				blocked.set(address, 'its placement has no heading section of its own');
				continue;
			}
			addCandidate(unit, move.to, {
				record: address,
				...(binding.element.tagName === 'ex-section-item' && { holds: move.holds }),
			});
		}
	}
	for (const [address, move] of request.deliverables) {
		const page = pages.get(move.destination.file);
		const element = page?.ids.get(move.destination.fragment);
		if (!page || !element) continue;
		if (request.ownedPages.get(page.file) !== projectOf(address)) {
			blocked.set(
				address,
				`its destination ${request.label(page.file)} belongs to another project`,
			);
			continue;
		}
		const unit = headingSection(page, element);
		if (!unit) {
			blocked.set(address, 'its destination has no heading section of its own');
			continue;
		}
		addCandidate(unit, move.to, { deliverable: address });
	}

	// A destination anchor named after its deliverable follows the deliverable's new id.
	const idRenames = new Map<string, string>();
	const renamedId = (file: string, id: string): string => idRenames.get(`${file}#${id}`) ?? id;
	for (const [address, move] of request.deliverables) {
		const oldId = localId(address);
		const newId = localId(move.newAddress);
		if (oldId !== newId && move.destination.fragment === oldId)
			idRenames.set(`${move.destination.file}#${oldId}`, newId);
	}

	const finalReferences = new Map<Binding, string>();

	// A unit moves only when everything placed in it goes to the same project.
	const incomingAnchors = new Map<string, Set<string>>();
	const accepted: CandidateUnit[] = [];
	// Outer sections first: a section inside one that moves goes with it.
	const ordered = [...candidates.values()].toSorted(
		(left, right) =>
			left.page.localeCompare(right.page) || left.start - right.start || right.end - left.end,
	);
	for (let index = 0; index < ordered.length; index += 1) {
		const unit = ordered[index];
		if (!unit) continue;
		if (
			accepted.some(
				(outer) => outer.page === unit.page && outer.start <= unit.start && outer.end >= unit.end,
			)
		)
			continue;
		const page = pages.get(unit.page);
		const targetFile = request.targetPages.get(unit.targetProject);
		const target = targetFile ? pages.get(targetFile) : undefined;
		const items = [...unit.records, ...unit.deliverables];
		const refuse = (reason: string): void => {
			for (const item of items) if (!blocked.has(item)) blocked.set(item, reason);
		};
		if (!page || !target || !targetFile) {
			refuse(`${unit.targetProject} has no page to receive it`);
			continue;
		}
		if (target.insertAt === undefined) {
			refuse(`${request.label(targetFile)} has no ex-document to receive it`);
			continue;
		}
		const placed = page.bindings
			.filter((binding) => binding.kind === 'placement' && within(unit, binding.start))
			.map((binding) => qualified(page.owner, binding.reference) ?? binding.reference);
		const destinations = new Set(
			placed.map((address) => request.records.get(address)?.to ?? 'stays'),
		);
		if (destinations.size > 1 || destinations.has('stays')) {
			const where = placed.map((address) => {
				const to = request.records.get(address)?.to;
				return to ? (maps.records.get(address) ?? address) : `${address} (stays)`;
			});
			refuse(
				`its section "${unit.heading}" places records that do not all go to one project: ${where.join(', ')}`,
			);
			continue;
		}
		const ownIds = [...page.ids].filter(([, element]) =>
			within(unit, element.sourceCodeLocation?.startOffset ?? -1),
		);
		const foreignDestination = request.stayingDestinations.find(
			(destination) =>
				destination.file === page.file && ownIds.some(([id]) => id === destination.fragment),
		);
		const otherMover = [...request.deliverables].find(
			([, move]) =>
				move.to !== unit.targetProject &&
				move.destination.file === page.file &&
				ownIds.some(([id]) => id === move.destination.fragment),
		);
		if (foreignDestination || otherMover) {
			refuse(
				`its section "${unit.heading}" is also the destination of a deliverable going elsewhere`,
			);
			continue;
		}
		const nested = page.text.slice(unit.start, unit.end);
		if (
			unit.kind === 'heading-section' &&
			/<ex-(?:section|deliverables|nav|initiative-overview)[\s>]/u.test(nested)
		) {
			refuse(`its section "${unit.heading}" holds page structure`);
			continue;
		}
		const missingHolds = [...unit.holds].filter((hold) => !target.sectionHolds.has(hold));
		if (missingHolds.length) {
			refuse(`${request.label(targetFile)} declares no section holding ${missingHolds.join(', ')}`);
			continue;
		}
		const arriving = incomingAnchors.get(targetFile) ?? new Set<string>();
		const idAnchors = ownIds.map(([id]) => renamedId(page.file, id));
		const taken = (anchor: string): boolean =>
			target.anchors.has(anchor) || arriving.has(anchor) || idAnchors.includes(anchor);
		const clash = idAnchors.find((anchor) => target.anchors.has(anchor) || arriving.has(anchor));
		if (clash) {
			if (unit.inner)
				ordered.splice(index + 1, 0, {
					kind: unit.kind,
					page: unit.page,
					heading: unit.heading,
					start: unit.inner.start,
					end: unit.inner.end,
					targetProject: unit.targetProject,
					records: unit.records,
					deliverables: unit.deliverables,
					holds: unit.holds,
				});
			else refuse(`${request.label(targetFile)} already has the anchor ${clash}`);
			continue;
		}
		// A placement whose bare id would clash with an authored id stays qualified.
		const unitReferences = new Map<Binding, string>();
		for (const binding of page.bindings) {
			if (binding.kind !== 'placement' || !within(unit, binding.start)) continue;
			const bare = rewriteReference(
				binding.reference,
				'record',
				{ before: page.owner, after: unit.targetProject },
				maps,
			);
			const reference =
				!bare.includes('/') && (taken(bare) || [...unitReferences.values()].includes(bare))
					? `${unit.targetProject}/${bare}`
					: bare;
			unitReferences.set(binding, reference);
		}
		for (const [binding, reference] of unitReferences) finalReferences.set(binding, reference);
		for (const anchor of [...idAnchors, ...unitReferences.values()]) arriving.add(anchor);
		incomingAnchors.set(targetFile, arriving);
		accepted.push(unit);
	}
	// An outer section that moves carries the sections inside it.
	const moving = accepted.filter(
		(unit) =>
			!accepted.some(
				(other) =>
					other !== unit &&
					other.page === unit.page &&
					other.start <= unit.start &&
					other.end >= unit.end,
			),
	);
	const movedRecords = new Map<CandidateUnit, string[]>();
	const movedDeliverables = new Map<CandidateUnit, string[]>();
	for (const unit of moving) {
		const page = pages.get(unit.page);
		if (!page) continue;
		movedRecords.set(
			unit,
			page.bindings.flatMap((binding) => {
				if (binding.kind !== 'placement' || !within(unit, binding.start)) return [];
				const address = qualified(page.owner, binding.reference);
				return address && request.records.has(address) ? [address] : [];
			}),
		);
		movedDeliverables.set(
			unit,
			[...request.deliverables].flatMap(([address, move]) => {
				const element =
					move.destination.file === page.file ? page.ids.get(move.destination.fragment) : undefined;
				return element && within(unit, element.sourceCodeLocation?.startOffset ?? -1)
					? [address]
					: [];
			}),
		);
	}
	const movedItems = new Set([...movedRecords.values(), ...movedDeliverables.values()].flat());

	/** A binding's reference after the run, written for the page it ends on. */
	const referenceAfter = (
		page: ParsedPage,
		binding: Binding,
		unit: CandidateUnit | undefined,
	): string => {
		const decided = unit ? finalReferences.get(binding) : undefined;
		if (decided) return decided;
		const after = unit ? unit.targetProject : page.owner;
		const reference = rewriteReference(
			binding.reference,
			'record',
			{ before: page.owner, after },
			maps,
		);
		const clashes =
			binding.kind === 'placement' &&
			!reference.includes('/') &&
			reference !== binding.reference &&
			page.ids.has(reference);
		return clashes && after ? `${after}/${reference}` : reference;
	};
	const anchors = new AnchorRelocations();
	for (const page of pages.values()) {
		const unitOf = (position: number): CandidateUnit | undefined =>
			moving.find((unit) => unit.page === page.file && within(unit, position));
		for (const binding of page.bindings) {
			if (binding.kind !== 'placement') continue;
			const unit = unitOf(binding.start);
			const reference = referenceAfter(page, binding, unit);
			const file = unit ? request.targetPages.get(unit.targetProject) : page.file;
			if (file && (file !== page.file || reference !== binding.reference))
				anchors.movePlacement(page.file, binding.reference, { file, fragment: reference });
		}
		for (const [id, element] of page.ids) {
			const unit = unitOf(element.sourceCodeLocation?.startOffset ?? -1);
			const file = unit ? request.targetPages.get(unit.targetProject) : undefined;
			if (file) anchors.move(page.file, id, { file, fragment: renamedId(page.file, id) });
		}
	}

	const edits = new Map<string, TextEdit[]>();
	const pushEdit = (file: string, edit: TextEdit): void => {
		const list = edits.get(file) ?? [];
		list.push(edit);
		edits.set(file, list);
	};
	const contentEdits = (
		page: ParsedPage,
		range: { start: number; end: number },
		after: string | undefined,
		fromAfter: string,
	): TextEdit[] => {
		const result: TextEdit[] = [];
		for (const binding of page.bindings) {
			if (!within(range, binding.start)) continue;
			const unit = moving.find(
				(candidate) => candidate.page === page.file && within(candidate, binding.start),
			);
			const reference =
				binding.kind === 'placement'
					? referenceAfter(page, binding, unit)
					: rewriteReference(binding.reference, 'record', { before: page.owner, after }, maps);
			if (reference !== binding.reference)
				result.push(attributeEdit(binding.attribute, reference, binding.start, binding.end));
		}
		for (const url of page.urls) {
			if (!within(range, url.start)) continue;
			const href = anchors.relocate(url.value, page.file, fromAfter);
			if (href !== url.value) result.push(attributeEdit(url.name, href, url.start, url.end));
		}
		if (fromAfter !== page.file)
			for (const [id, element] of page.ids) {
				const span = element.sourceCodeLocation?.attrs?.['id'];
				const renamed = renamedId(page.file, id);
				if (span && renamed !== id && within(range, span.startOffset))
					result.push(attributeEdit('id', renamed, span.startOffset, span.endOffset));
			}
		return result;
	};
	const insertions = new Map<string, Map<string, string[]>>();
	for (const page of pages.values()) {
		const own = moving.filter((unit) => unit.page === page.file);
		const outside = contentEdits(
			page,
			{ start: 0, end: page.text.length },
			page.owner,
			page.file,
		).filter((edit) => !own.some((unit) => within(unit, edit.start)));
		for (const edit of outside) pushEdit(page.file, edit);
		for (const unit of own) {
			const targetFile = request.targetPages.get(unit.targetProject);
			if (!targetFile) continue;
			pushEdit(page.file, { start: unit.start, end: unit.end, text: '' });
			const inner = contentEdits(page, unit, unit.targetProject, targetFile).map((edit) => ({
				text: edit.text,
				start: edit.start - unit.start,
				end: edit.end - unit.start,
			}));
			const text = applyTextEdits(page.text.slice(unit.start, unit.end), inner).replace(
				/\s+$/u,
				'',
			);
			const byTarget = insertions.get(targetFile) ?? new Map<string, string[]>();
			const source = page.owner ?? 'initiative';
			byTarget.set(source, [...(byTarget.get(source) ?? []), text]);
			insertions.set(targetFile, byTarget);
		}
	}
	for (const [targetFile, groups] of insertions) {
		const target = pages.get(targetFile);
		if (!target || target.insertAt === undefined) continue;
		const blocks = [...groups].map(([source, texts]) => {
			let id = `from-${source}`;
			for (let counter = 2; target.anchors.has(id); counter += 1) id = `from-${source}-${counter}`;
			return [`<h2 id="${escapeHtml(id)}">From ${escapeHtml(source)}</h2>`].concat(texts);
		});
		const lineStart = target.text.lastIndexOf('\n', target.insertAt - 1) + 1;
		const indent = /^[ \t]*/u.exec(target.text.slice(lineStart, target.insertAt))?.[0] ?? '';
		pushEdit(targetFile, {
			start: target.insertAt,
			end: target.insertAt,
			text: `${blocks.flat().join(`\n${indent}`)}\n${indent}`,
		});
	}

	const files = new Map<string, string>();
	for (const [file, list] of edits) {
		const page = pages.get(file);
		if (!page || !list.length) continue;
		const text = applyTextEdits(page.text, list);
		if (text !== page.text) files.set(file, text);
	}
	for (const address of request.records.keys()) {
		const reason = blocked.get(address);
		if (movedItems.has(address) || !reason) continue;
		unplaced.push({ address, file: placementFiles.get(address) ?? '', reason });
	}
	const destinations = new Map<string, AnchorTarget>();
	for (const [address, move] of request.deliverables) {
		destinations.set(address, anchors.target(move.destination.file, move.destination.fragment));
		if (!movedItems.has(address) && blocked.has(address))
			unplaced.push({ address, file: move.destination.file, reason: blocked.get(address) ?? '' });
	}
	const moved = moving.map((unit): MovedPageUnit => ({
		kind: unit.kind,
		page: unit.page,
		start: unit.start,
		end: unit.end,
		heading: unit.heading,
		target: request.targetPages.get(unit.targetProject) ?? '',
		targetProject: unit.targetProject,
		records: movedRecords.get(unit) ?? [],
		deliverables: movedDeliverables.get(unit) ?? [],
	}));
	const recordPlacements = new Map<string, string>();
	for (const [address, file] of placementFiles) {
		const unit = moving.find((candidate) => movedRecords.get(candidate)?.includes(address));
		recordPlacements.set(
			address,
			unit ? (request.targetPages.get(unit.targetProject) ?? file) : file,
		);
	}
	return { files, anchors, moved, unplaced, destinations, recordPlacements };
}
