// The SDK's public types: generated Rust contracts, readonly at the API edge.
import type * as Contracts from './contracts/contracts.ts';
import type { DeepReadonly } from './readonly-values.ts';

/** A complete configuration: the single execution authority for a call. */
export type Configuration = DeepReadonly<Contracts.Configuration>;
/** A configuration whose definitions were all captured by `load`. */
export type ResolvedConfiguration = DeepReadonly<Contracts.ResolvedConfiguration>;
/** A loaded configuration plus the informational receipt of what `load` read. */
export type LoadedConfiguration = DeepReadonly<Contracts.LoadedConfiguration>;
/** Either form is accepted; a loaded value's `sources` receipt never executes. */
export type ConfigurationInput = Configuration | LoadedConfiguration;

export type LoadOptions = DeepReadonly<Contracts.LoadOptions>;
export type Scope = DeepReadonly<Contracts.Scope>;
export type ContractInfo = DeepReadonly<Contracts.ContractInfo>;

export type LoadOutcome = DeepReadonly<Contracts.LoadOutcome>;
export type CheckOutcome = DeepReadonly<Contracts.CheckOutcome>;
export type RunOutcome = DeepReadonly<Contracts.RunOutcome>;

export type CompletedReport = DeepReadonly<Contracts.SdkReport<Contracts.CompletedStatus>>;
export type InvalidReport = DeepReadonly<Contracts.SdkReport<Contracts.InvalidStatus>>;
export type IncompleteReport = DeepReadonly<Contracts.SdkReport<Contracts.IncompleteStatus>>;
export type CancelledReport = DeepReadonly<Contracts.SdkReport<Contracts.CancelledStatus>>;
export type SdkReport = DeepReadonly<Contracts.SdkReport<Contracts.LintStatus>>;
export type RulePieceResult = DeepReadonly<Contracts.RulePieceResult>;
export type RulePieceEnding = DeepReadonly<Contracts.RulePieceEnding>;
export type WorkUnavailableReason = DeepReadonly<Contracts.WorkUnavailableReason>;
export type UnavailableCounts = DeepReadonly<Contracts.UnavailableCounts>;

export type Diagnostic = DeepReadonly<Contracts.Diagnostic>;
export type DiagnosticCode = Contracts.DiagnosticCode;
export type SdkFailure = DeepReadonly<Contracts.SdkFailure>;
export type CheckSummary = DeepReadonly<Contracts.CheckSummary>;
export type DefinitionOrigin = DeepReadonly<Contracts.DefinitionOrigin>;
export type SourceLocator = DeepReadonly<Contracts.SourceLocator>;
export type DefinitionSnapshot = DeepReadonly<Contracts.DefinitionSnapshot>;
export type RulebookDefinition = DeepReadonly<Contracts.RulebookDefinition>;
export type EvidenceDefinitions = DeepReadonly<Contracts.EvidenceDefinitions>;
export type RuleEntry = DeepReadonly<Contracts.RuleEntryWire>;
export type DecisionMechanicalTool = DeepReadonly<Contracts.DecisionMechanicalTool>;
export type CountBound = DeepReadonly<Contracts.CountBound>;
export type MeasureFields = DeepReadonly<Contracts.MeasureFields>;
export type EvidenceKindDefinition = DeepReadonly<Contracts.EvidenceKindWire>;
export type FilterDefinition = DeepReadonly<Contracts.FilterWire>;
/** Where an evidence field's text comes from; section and context text modes include `heading`. */
export type FieldSource = DeepReadonly<Contracts.FieldSource>;
export type DocumentCandidates = DeepReadonly<Contracts.DocumentCandidates>;
export type SectionScope = DeepReadonly<Contracts.SectionScope>;
export type SectionText = Contracts.SectionText;
export type ContextText = Contracts.ContextText;
/** A built field, including a selected document or missing document candidates. */
export type FieldValue = DeepReadonly<Contracts.FieldValue>;
export type SdkFinding = DeepReadonly<Contracts.SdkFinding>;
export type EvidenceLanguage = Contracts.EvidenceLanguage;
/** HTML conventions: tags left out of section outlines, attributes read as asking text. */
export type HtmlEvidenceSettings = DeepReadonly<Contracts.HtmlEvidenceSettings>;
export type HtmlMarkdownIsland = DeepReadonly<Contracts.HtmlMarkdownIsland>;
/** Markdown conventions: the heading-anchor scheme, if any. */
export type MarkdownEvidenceSettings = DeepReadonly<Contracts.MarkdownEvidenceSettings>;
export type HeadingAnchorScheme = Contracts.HeadingAnchorScheme;
export type HtmlTagName = Contracts.HtmlTagName;
export type HtmlAttributeName = Contracts.HtmlAttributeName;
/** A target fragment's state; an unsupported fragment names why. */
export type TargetAnchor = DeepReadonly<Contracts.TargetAnchor>;
export type TargetAnchorUnsupportedReason = Contracts.TargetAnchorUnsupportedReason;
export type { AstMatcher, AstUtility, AuthoredLanguage } from './contracts/ast-grep.ts';

/** Per-call options. Aborting one call's signal cancels only that call. */
export interface CallOptions {
	readonly signal?: AbortSignal;
}

/** A run selects an absolute checked root, scope and optional label-free cases. */
export interface RunOptions extends DeepReadonly<Contracts.RunRequest>, CallOptions {}
export type CaseBinding = DeepReadonly<Contracts.CaseBinding>;
export type CaseSubjectBinding = DeepReadonly<Contracts.CaseSubjectBinding>;
export type CaseBindingStatus = DeepReadonly<Contracts.CaseBindingStatus>;
export type CaseBindingResult = DeepReadonly<Contracts.CaseBindingResult>;
export type CaseExecutionOwner = DeepReadonly<Contracts.CaseExecutionOwner>;
export type CaseSelectionReport = DeepReadonly<Contracts.CaseSelectionReport>;

/** Inspection status describes evidence acquisition and selection, never rule findings. */
export type InspectionStatus = DeepReadonly<Contracts.InspectionStatus>;
export type InspectionOutcome = DeepReadonly<Contracts.InspectionOutcome>;
export type InspectionReport = DeepReadonly<Contracts.InspectionReport<Contracts.InspectionStatus>>;
export type SdkDocumentValue = DeepReadonly<Contracts.SdkDocumentValue>;
export type SdkDocument = DeepReadonly<Contracts.SdkDocument<Contracts.SdkDocumentValue>>;
export type DeserializationOutcome = DeepReadonly<Contracts.DeserializationOutcome>;
export type SdkDocumentDiagnostic = DeepReadonly<Contracts.SdkDocumentDiagnostic>;

/** The pure writer refuses unsupported data without loading the native addon. */
export type SerializationOutcome =
	| { readonly kind: 'serialized'; readonly json: string }
	| {
			readonly kind: 'rejected';
			readonly reason: 'unsupported_runtime' | 'invalid_data';
			readonly message: string;
	  };

/** Inspect one configured evidence kind, or all kinds when omitted. */
export interface InspectOptions extends Pick<RunOptions, 'root' | 'scope' | 'paths'>, CallOptions {
	readonly evidence_kind?: string;
}
