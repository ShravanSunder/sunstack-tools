import type { ProjectState } from './initiative-view.ts';

/** Plain-language meaning of each deliverable state, shown wherever a state appears. */
export const deliverableStateGlossary = {
	planned: 'Designed and ready to build; no implementation task is done yet.',
	'implementation-in-progress': 'Being built: some implementation tasks are done, or no build is recorded yet.',
	'design-blocked': 'Waiting on an open owner decision or an unsettled design item this deliverable relies on.',
	'dependency-blocked': 'Waiting for a deliverable it depends on to be accepted.',
	'awaiting-verification': 'Built; its planned checks have not all passed or been run yet.',
	'verification-blocked': 'A planned check cannot run until something it needs is in place.',
	'verification-failed': 'A planned check ran and failed.',
	'ready-for-acceptance': 'Built and every planned check passed; waiting for the Lead to accept it.',
	accepted: 'Built, checked and accepted.',
	later: 'Named for later; not designed or checked now.',
} as const;

export type GlossaryDeliverableState = keyof typeof deliverableStateGlossary;

/** Plain-language meaning of each project state in an initiative. */
export const projectStateGlossary: Readonly<Record<ProjectState, string>> = {
	accepted: 'Every core deliverable in this project is accepted.',
	'dependency-blocked': 'Waiting for a project it depends on to finish.',
	'design-blocked': 'A core deliverable is waiting on an open owner decision.',
	'in-progress': 'Work has started on at least one core deliverable.',
	planned: 'Ready to start; no core deliverable has started.',
	later: 'Only later deliverables; nothing is designed or checked now.',
};

/** What each record kind is, for the reader who meets its id. */
export const recordKindGlossary = {
	need: 'Need: what someone must be able to do, in their words.',
	rule: 'Rule: one sentence the system must keep, with cases that test it.',
	realization: 'Realization: how a part of the system carries out the rules it names.',
	'planned-check': 'Planned check: how a rule will be shown to hold, with its cases.',
	choice: 'Choice: a decision between credible options, open or decided.',
	ruling: 'Ruling: a decision with no alternatives to weigh.',
	assumption: 'Assumption: something the design treats as true that nobody has checked or authorized.',
	deliverable: 'Deliverable: a piece of work that is built, checked and accepted.',
} as const;

/** The id prefixes authors use, by convention, for each record kind. */
export const idPrefixGlossary = {
	U: recordKindGlossary.need,
	R: recordKindGlossary.rule,
	M: recordKindGlossary.realization,
	V: recordKindGlossary['planned-check'],
	D: 'Decision: a choice or a ruling.',
	A: recordKindGlossary.assumption,
	DEL: recordKindGlossary.deliverable,
} as const;
