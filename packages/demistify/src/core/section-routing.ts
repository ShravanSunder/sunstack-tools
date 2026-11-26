export const sectionHolds = ['needs-owner', 'decided-by-agent', 'settled', 'assumed'] as const;
export type SectionHold = (typeof sectionHolds)[number];
