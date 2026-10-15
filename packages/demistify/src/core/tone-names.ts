/** The ten tones and the synonyms models often write for them. Pure data, shared by runtime and checker. */
export const toneNames = [
	'note',
	'tldr',
	'decision',
	'risk',
	'question',
	'done',
	'planned',
	'changed',
	'added',
	'removed',
] as const;
export type ToneName = (typeof toneNames)[number];

export const toneAliases: Readonly<Record<string, ToneName>> = {
	info: 'note',
	tip: 'note',
	summary: 'tldr',
	warning: 'risk',
	danger: 'risk',
	error: 'risk',
	blocking: 'risk',
	success: 'done',
	complete: 'done',
	todo: 'planned',
	new: 'added',
	deleted: 'removed',
	modified: 'changed',
};
