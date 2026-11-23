import { subjectIcons } from '../../core/subject-icons.ts';

const declarations = `// Authoring types for Demistify design, decision and assumption files.
type SubjectIcon = ${subjectIcons.map((icon): string => JSON.stringify(icon)).join(' | ')};
type Target = { page: string; fragment?: string };
type NavigationTarget<TRecordId extends string> = Target | { record: TRecordId };
type Page = { id: string; title: string; href: string; icon: SubjectIcon };
type MapNode<TRecordId extends string> = {
  title?: string;
  description: string;
  icon: SubjectIcon;
  target?: NavigationTarget<TRecordId>;
  children?: readonly MapNode<TRecordId>[];
};
type Walkthrough<TRecordId extends string> = {
  id: string;
  title: string;
  description: string;
  steps: readonly { target: NavigationTarget<TRecordId>; narration: string; icon?: SubjectIcon }[];
};
type SourceReference = { href: string; excerpt: string; verbatim?: true };
type RecordFields = { id: string; title?: string; statement: string };
type RecordReference = string | readonly string[];
type DesignDependencies = { governedBy?: readonly string[]; reliesOn?: readonly string[] };
type Cases = readonly { given: string; expect: string }[];
type SettledDecision = { decision: string; decidedOn: string };
type Authority =
  | ({ state: 'owner-decided'; owner: 'owner'; sourceKind: 'owner-words' | 'owner-accepted' } & SettledDecision)
  | { state: 'owner-open'; owner: 'owner'; question: string }
  | ({ state: 'lead-decided'; lead: string; sourceKind: 'author' | 'contract'; rationale: string } & SettledDecision);
export type DesignRecord = RecordFields & (
  | { kind: 'need' }
  | ({ kind: 'rule'; serves?: RecordReference; cases?: Cases } & DesignDependencies)
  | ({ kind: 'realization'; realizes: RecordReference } & DesignDependencies)
  | ({ kind: 'planned-check'; checks: RecordReference; journey: string; unverified: string; cases: Cases } & DesignDependencies)
);
export type DesignDecision = Omit<RecordFields, 'statement'> & { kind: 'choice'; authority: Authority; supersedes?: string; sources?: readonly SourceReference[] };
export type DesignAssumption = RecordFields & { kind: 'assumption'; sources?: readonly SourceReference[] } & (
  { status: 'open'; resolution?: never } | { status: 'confirmed' | 'refuted'; resolution: SourceReference }
);
export type DesignCore<TRecordId extends string = KnownRecordId> = {
  pages: readonly Page[];
  mapGroups: readonly MapNode<TRecordId>[];
  walkthroughs?: readonly Walkthrough<TRecordId>[];
  records: readonly DesignRecord[];
  acceptances?: readonly { deliverableId: string; build: string; evidence: string }[];
};
`;

export function designTypesFor(recordIds: readonly string[]): string {
	const ids = [...new Set(recordIds)].toSorted().map((id): string => JSON.stringify(id));
	return `${declarations}export type KnownRecordId = ${ids.join(' | ') || 'never'};\n`;
}

export const designTypeSource: string = designTypesFor([]);
