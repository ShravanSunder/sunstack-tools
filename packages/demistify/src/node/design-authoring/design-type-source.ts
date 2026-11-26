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
type SourceReference = { href: string; excerpt: string } & ({ verbatim?: true; shown?: never } | { verbatim: true; shown: string });
type RecordFields = { id: string; title?: string; statement: string };
type RecordReference = string | readonly string[];
type DesignDependencies = { governedBy?: readonly string[]; reliesOn?: readonly string[] };
type Cases = readonly { given: string; expect: string }[];
type Weight = 'design' | 'process' | 'minor';
type AgentActor = { alias: string; role: 'lead' | 'implementer' | 'verifier' | 'reviewer' | 'advisor' };
type SettledAuthority =
  | { state: 'owner-decided'; sourceKind: 'owner-words' | 'owner-accepted'; decidedOn: string; source: SourceReference }
  | ({ state: 'agent-decided'; by: AgentActor; rationale: string; decidedOn: string; weight: Weight } &
      ({ sourceKind: 'contract'; source: SourceReference } | { sourceKind: 'author' }));
type ChoiceOption = { id: string; title: string };
export type DesignRecord = RecordFields & (
  | ({ kind: 'need' } & DesignDependencies)
  | ({ kind: 'rule'; serves?: RecordReference; cases?: Cases } & DesignDependencies)
  | ({ kind: 'realization'; realizes: RecordReference } & DesignDependencies)
  | ({ kind: 'planned-check'; checks: RecordReference; journey: string; unverified: string; cases: Cases } & DesignDependencies)
);
type DecisionIdentity = Omit<RecordFields, 'statement'> & { supersedes?: string; sources?: readonly SourceReference[] };
export type DesignDecision = DecisionIdentity & (
  | { kind: 'choice'; question: string; options: readonly [ChoiceOption, ChoiceOption, ...ChoiceOption[]];
      recommended: 'none' | { option: string; by: AgentActor };
      authority: { state: 'owner-open' } | (SettledAuthority & { chosen: string; decision: string }) }
  | { kind: 'ruling'; statement: string; authority: SettledAuthority }
);
export type DesignAssumption = RecordFields & { kind: 'assumption'; by: AgentActor; weight: Weight; sources?: readonly SourceReference[] } & (
  { status: 'open'; settles: string } | { status: 'confirmed' | 'refuted'; resolution: SourceReference; settledBy: string }
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
