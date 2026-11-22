import type {
	NavigationTarget,
	SharedMapNode,
	SharedNavigation,
} from '../../core/shared-navigation.ts';
import { normalizeExcerpt, type PageMarkup } from './design-page-markup.ts';
import type {
	AuthoredMapNode,
	AuthoredNavigationTarget,
	DesignCore,
	DesignRecord,
} from './schema.ts';

interface ResolvedRecord {
	readonly record: DesignRecord;
	readonly file: string;
}

type GeneratedNavigation = Omit<SharedNavigation, 'walkthroughs'> &
	Partial<Pick<SharedNavigation, 'walkthroughs'>>;

export function compileDesignNavigation(
	design: Pick<DesignCore, 'pages' | 'mapGroups' | 'walkthroughs'>,
	targetForRecord: (reference: string) => NavigationTarget,
): GeneratedNavigation {
	function resolveTarget(target: AuthoredNavigationTarget): NavigationTarget {
		return 'page' in target ? target : targetForRecord(target.record);
	}

	function resolveMapNode(node: AuthoredMapNode): SharedMapNode {
		const { target, children, ...label } = node;
		return {
			...label,
			...(target && { target: resolveTarget(target) }),
			children: children.map(resolveMapNode),
		};
	}

	return {
		version: 1,
		pages: design.pages,
		map: design.mapGroups.map(resolveMapNode),
		...(design.walkthroughs === undefined
			? {}
			: {
					walkthroughs: design.walkthroughs.map((route) => ({
						...route,
						steps: route.steps.map((step) => ({
							...step,
							target: resolveTarget(step.target),
						})),
					})),
				}),
	};
}

export function checkDecisionChoices(options: {
	readonly choices: PageMarkup['decisionChoices'];
	readonly file: string;
	readonly displayPath: string;
	readonly resolveRecord: (reference: string) => ResolvedRecord;
}): void {
	const { choices, file, displayPath, resolveRecord } = options;
	for (const { id, reference, title } of choices) {
		const { record, file: decisionFile } = resolveRecord(reference);
		if (record.kind !== 'choice' || record.authority.state !== 'owner-open')
			throw new Error(
				`${displayPath}: Choice ${id} must bind to an open owner decision: ${reference}.`,
			);
		if (decisionFile !== file || record.destination.fragment !== id)
			throw new Error(`${displayPath}: Choice ${id} must be the HTML destination of ${reference}.`);
		if (normalizeExcerpt(title) !== normalizeExcerpt(record.authority.question))
			throw new Error(
				`${displayPath}: Choice ${id} must ask the question recorded in ${reference}.`,
			);
	}
}
