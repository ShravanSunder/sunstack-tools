import { z } from 'zod';

const planningNodeSchema = z.strictObject({
	id: z.string(),
	label: z.string(),
	eyebrow: z.string(),
	subtitle: z.string(),
	x: z.number(),
	y: z.number(),
	tone: z.enum(['strong', 'change', 'keep']),
	purpose: z.string(),
	boundary: z.string(),
	source: z.string(),
	sourceUrl: z.string(),
});
const planningViewSchema = z.strictObject({
	title: z.string(),
	description: z.string(),
	height: z.number(),
	nodes: z.array(planningNodeSchema),
	edges: z.array(
		z.union([z.tuple([z.string(), z.string()]), z.tuple([z.string(), z.string(), z.string()])]),
	),
});
export const planningViewsSchema = z.strictObject({
	roadmap: planningViewSchema,
	design: planningViewSchema,
	roles: planningViewSchema,
});
export type PlanningView = z.infer<typeof planningViewSchema>;
export type PlanningViews = z.infer<typeof planningViewsSchema>;

export function planningMermaid(view: PlanningView): string {
	return `flowchart TD\n${view.nodes.map((node): string => `  ${node.id}[${JSON.stringify(node.label)}]`).join('\n')}\n${view.edges.map((edge): string => `  ${edge[0]} ${edge[2] ? '<-.->|facts|' : '-->'} ${edge[1]}`).join('\n')}`;
}

export function themedPlanningMermaid(view: PlanningView): string {
	const classes = ['strong', 'change', 'keep']
		.map(
			(tone): string =>
				`  classDef ${tone} fill:var(--${tone}-bg),stroke:var(--${tone}),color:var(--ink)\n  class ${view.nodes
					.filter((node): boolean => node.tone === tone)
					.map((node): string => node.id)
					.join(',')} ${tone}`,
		)
		.filter((line): boolean => !line.includes('class  '));
	return `${planningMermaid(view)}\n${classes.join('\n')}\n`;
}

export function planningD2(
	view: PlanningView,
	themeColors: Readonly<Record<string, string>>,
	detailLinks: 'linked' | 'plain' = 'linked',
): string {
	const nodeDeclarations = view.nodes.map((node): string => {
		const fillColor = themeColors[`${node.tone}-bg`];
		const strokeColor = themeColors[node.tone];
		const detailLink = detailLinks === 'linked' ? `${node.id}.link: "#${node.id}"\n` : '';
		return `${node.id}: ${JSON.stringify(node.label)}\n${detailLink}${node.id}.style.fill: "${fillColor}"\n${node.id}.style.stroke: "${strokeColor}"\n${node.id}.style.font-color: "${themeColors['ink']}"\n${node.id}.style.font-size: 20`;
	});
	const edgeDeclarations = view.edges.map((edge): string =>
		edge[2]
			? `${edge[0]} <-> ${edge[1]}: {label: facts; style.stroke-dash: 3}`
			: `${edge[0]} -> ${edge[1]}`,
	);
	return `grid-columns: ${view.nodes.length === 9 ? 3 : 2}\ngrid-gap: 80\n${nodeDeclarations.join('\n')}\n${edgeDeclarations.join('\n')}\n`;
}
