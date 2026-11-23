import type {
	NavigationTarget,
	SharedMapNode,
	SharedNavigation,
} from '../../core/shared-navigation.ts';
import {
	recordLabel,
	type AuthoredMapNode,
	type AuthoredNavigationTarget,
	type DesignCore,
	type DesignRecord,
} from './schema.ts';

interface RecordNavigation {
	readonly record: DesignRecord;
	readonly target?: NavigationTarget | undefined;
}

export function compileDesignNavigation(
	design: Pick<DesignCore, 'pages' | 'mapGroups' | 'walkthroughs'>,
	navigationForRecord: (reference: string) => RecordNavigation,
): SharedNavigation {
	function resolveTarget(target: AuthoredNavigationTarget): NavigationTarget | undefined {
		return 'page' in target ? target : navigationForRecord(target.record).target;
	}

	function resolveMapNode(node: AuthoredMapNode): SharedMapNode {
		const { title, target, children, ...label } = node;
		const record =
			target && 'record' in target ? navigationForRecord(target.record).record : undefined;
		const resolvedTitle = title ?? (record && recordLabel(record));
		if (!resolvedTitle) throw new Error('A Map node without a record needs a title.');
		const destination = target && resolveTarget(target);
		return {
			...label,
			title: resolvedTitle,
			...(destination && { target: destination }),
			children: children.map(resolveMapNode),
		};
	}

	return {
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
	};
}
