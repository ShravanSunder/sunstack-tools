import {
	sharedNavigationSchema,
	type NavigationTarget,
	type SharedMapNode,
	type SharedNavigation,
} from '../../core/shared-navigation.ts';
import { qualifyRecord } from './record-projection.ts';
import {
	recordLabel,
	type AuthoredMapNode,
	type AuthoredNavigationTarget,
	type DesignCore,
	type DesignRecord,
} from './schema.ts';

interface NavigationRecords {
	readonly source: string;
	readonly records: ReadonlyMap<string, DesignRecord>;
	readonly namespace?: string | undefined;
}

export function compileDesignNavigation(
	design: Pick<DesignCore, 'pages' | 'mapGroups' | 'walkthroughs'>,
	{ source, records, namespace }: NavigationRecords,
	targetForRecord?: (id: string) => NavigationTarget | undefined,
): SharedNavigation {
	function resolveRecord(reference: string): DesignRecord {
		if (!namespace && reference.includes('/'))
			throw new Error(`Qualified reference ${reference} requires demistify initiative check.`);
		const record = records.get(qualifyRecord(namespace, reference));
		if (!record) throw new Error(`${source}: unknown design record ${reference}.`);
		return record;
	}

	function resolveTarget(target: AuthoredNavigationTarget): NavigationTarget | undefined {
		if ('page' in target) return target;
		const record = resolveRecord(target.record);
		return targetForRecord?.(record.id);
	}

	function resolveMapNode(node: AuthoredMapNode): SharedMapNode {
		const { title, target, children, ...label } = node;
		const record = target && 'record' in target ? resolveRecord(target.record) : undefined;
		const resolvedTitle = title ?? (record && recordLabel(record));
		if (!resolvedTitle) throw new Error('A Map node without a record needs a title.');
		const destination = record ? targetForRecord?.(record.id) : target && resolveTarget(target);
		return {
			...label,
			title: resolvedTitle,
			...(destination && { target: destination }),
			children: children.map(resolveMapNode),
		};
	}

	return sharedNavigationSchema.parse({
		version: 1,
		pages: design.pages,
		map: design.mapGroups.map(resolveMapNode),
		walkthroughs: (design.walkthroughs ?? []).map(({ id, title, description, steps }) => ({
			id,
			title,
			description,
			steps: steps.map(({ target, narration, icon }) => ({
				narration,
				icon,
				target: resolveTarget(target),
			})),
		})),
	});
}
