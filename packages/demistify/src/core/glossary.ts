import type { ProjectState } from './initiative-view.ts';

/** Plain-language meaning of each deliverable state, shown wherever a state appears. */
export const deliverableStateGlossary = {
	planned: 'Designed and ready to build; no issue is closed yet.',
	'implementation-in-progress': 'Being built: some issues are closed, or no build is recorded yet.',
	'design-blocked':
		'Waiting on an open owner decision or an unsettled design item this deliverable relies on.',
	'dependency-blocked': 'Blocked until a deliverable it names in blockedBy is accepted.',
	'awaiting-verification': 'Built; its planned checks have not all passed or been run yet.',
	'verification-blocked': 'A planned check cannot run until something it needs is in place.',
	'verification-failed': 'A planned check ran and failed.',
	'ready-for-acceptance':
		'Built and every planned check passed; waiting for the Lead to accept it.',
	accepted: 'Built, checked and accepted.',
	later: 'Named for later; not designed or checked now.',
} as const;

/** What each level of a typed plan is, and what a project's number means. */
export const levelGlossary = {
	initiative:
		'Initiative: the whole plan. It holds its projects, each with a number, and the decisions they share.',
	project:
		'Project: one piece of work that finishes, with its own design. It says what it achieves and what the owner checks to accept it.',
	deliverable:
		'Deliverable: a piece of a project that is built, checked and accepted. It can be blocked by deliverables in any project.',
	number:
		'Lower numbers come first. Projects with the same number can run side by side. Number 0 can show finished groundwork.',
	blockedBy: 'Blocked by: this deliverable cannot be accepted before the deliverables it names.',
} as const;

/** Plain-language meaning of each project state in an initiative. */
export const projectStateGlossary: Readonly<Record<ProjectState, string>> = {
	accepted: 'Every core deliverable in this project is accepted.',
	'dependency-blocked': 'Every open core deliverable here waits on a deliverable it depends on.',
	'design-blocked': 'Every open core deliverable here waits on an open owner decision.',
	'in-progress': 'Work has started on at least one core deliverable.',
	planned: 'A core deliverable can start; none has started.',
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
	assumption:
		'Assumption: something the design treats as true that nobody has checked or authorized.',
	deliverable: levelGlossary.deliverable,
} as const;

/** The id prefixes authors use, by convention, for each record kind. */
export type IdPrefix = 'U' | 'R' | 'M' | 'V' | 'D' | 'A' | 'DEL';
export const idPrefixGlossary: ReadonlyMap<IdPrefix, string> = new Map<IdPrefix, string>([
	['U', recordKindGlossary.need],
	['R', recordKindGlossary.rule],
	['M', recordKindGlossary.realization],
	['V', recordKindGlossary['planned-check']],
	['D', 'Decision: a choice or a ruling.'],
	['A', recordKindGlossary.assumption],
	['DEL', recordKindGlossary.deliverable],
]);

/** What each planned check's result means. */
export const checkStateGlossary = {
	missing: 'No result is recorded for this check yet.',
	stale: 'A result is recorded, but for an older build than the current candidate.',
	passed: 'The check ran on the current build and passed.',
	failed: 'The check ran on the current build and failed.',
	blocked: 'The check could not run; the reason is recorded.',
} as const;

/** What each issue's state means. */
export const issueStateGlossary = {
	closed: 'Recorded as done in the current candidate.',
	open: 'Not done yet.',
} as const;

/** What a decision's status means. */
export const decisionStatusGlossary = {
	Open: 'Waiting for the owner to decide.',
	Decided: 'Decided, by the owner or an agent as the record names.',
	Superseded: 'Replaced by a later decision, which the record links.',
} as const;

/** What an assumption's state means. */
export const assumptionStateGlossary = {
	open: 'Taken as true but not yet checked; the record names what would settle it.',
	confirmed: 'Checked and found true; the record names who settled it and how.',
	refuted: 'Checked and found false; work relying on it must change.',
} as const;
