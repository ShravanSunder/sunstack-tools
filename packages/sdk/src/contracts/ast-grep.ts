// Generated from the pinned ast-grep 0.51.0 transport schema. Do not edit.

/** JSON-representable data. */
export type JsonData =
	| null
	| boolean
	| number
	| string
	| readonly JsonData[]
	| { readonly [key: string]: JsonData };

/**
 * The declared transport shape of a matcher: the pinned upstream core composed with
 * every accepted wrapper property. Used only to generate declarations; values are
 * decoded by the upstream decoder into [`AstMatcher`].
 */
export type AstMatcher = {
	/**
	 * Additional meta variables pattern to filter matching
	 */
	readonly constraints?: Readonly<Record<string, SerializableRule>> | null;
	readonly files?: readonly RuleFileGlob[] | null;
	readonly fix?: SerializableFixer;
	readonly ignores?: readonly RuleFileGlob[] | null;
	readonly labels?: Readonly<Record<string, LabelConfig>> | null;
	readonly language: AuthoredLanguage;
	readonly message?: string | null;
	readonly metadata?: Readonly<Record<string, JsonData>> | null;
	readonly note?: string | null;
	readonly rewriters?: readonly SerializableRewriter[] | null;
	/**
	 * A rule object to find matching AST nodes
	 */
	readonly rule: SerializableRule;
	readonly severity?: Severity | null;
	/**
	 * A dictionary for metavariable manipulation. Dict key is the new variable name.
	 * Dict value is a [transformation] that specifies how meta var is processed.
	 * See [transformation doc](https://ast-grep.github.io/reference/yaml/transformation.html).
	 */
	readonly transform?: Readonly<Record<string, Transformation>> | null;
	readonly url?: string | null;
	/**
	 * Utility rules that can be used in `matches`
	 */
	readonly utils?: Readonly<Record<string, SerializableRule>> | null;
};

/**
 * The declared transport shape of a global utility; see [`AstMatcherSchema`].
 */
export type AstUtility = {
	readonly arguments?: readonly string[] | null;
	/**
	 * Additional meta variables pattern to filter matching
	 */
	readonly constraints?: Readonly<Record<string, SerializableRule>> | null;
	readonly language: AuthoredLanguage;
	/**
	 * A rule object to find matching AST nodes
	 */
	readonly rule: SerializableRule;
	/**
	 * A dictionary for metavariable manipulation. Dict key is the new variable name.
	 * Dict value is a [transformation] that specifies how meta var is processed.
	 * See [transformation doc](https://ast-grep.github.io/reference/yaml/transformation.html).
	 */
	readonly transform?: Readonly<Record<string, Transformation>> | null;
	/**
	 * Utility rules that can be used in `matches`
	 */
	readonly utils?: Readonly<Record<string, SerializableRule>> | null;
};

/**
 * Canonical upstream language spellings accepted by the pinned decoder.
 *
 * Distinct from the snake_case `EvidenceLanguage` in results.
 */
export type AuthoredLanguage =
	| 'Bash'
	| 'C'
	| 'Cpp'
	| 'CSharp'
	| 'Css'
	| 'Dart'
	| 'Elixir'
	| 'Go'
	| 'Haskell'
	| 'Hcl'
	| 'Html'
	| 'Java'
	| 'JavaScript'
	| 'Json'
	| 'Kotlin'
	| 'Lua'
	| 'Markdown'
	| 'Nix'
	| 'Php'
	| 'Python'
	| 'Ruby'
	| 'Rust'
	| 'Scala'
	| 'Solidity'
	| 'Swift'
	| 'Tsx'
	| 'TypeScript'
	| 'Yaml'
	| 'Zig';

/**
 * Converts the source meta variable's text content to a specified case format.
 */
export type Convert = {
	/**
	 * optional separators to specify how to separate word
	 */
	readonly separatedBy?: readonly Separator[] | null;
	/**
	 * source meta variable to be transformed
	 */
	readonly source: string;
	/**
	 * the target case format to convert the text content to
	 */
	readonly toCase: StringCase;
};

/**
 * Upstream `LabelConfig`, which is not exported by the pinned crate.
 */
export type LabelConfig = {
	readonly message?: string | null;
	readonly style: LabelStyle;
};

export type LabelStyle = 'primary' | 'secondary';

/**
 * A string or number describing the indices of matching nodes in a list of siblings.
 */
export type NthChildSimple = number | string;

/**
 * A String pattern will match one single AST node according to pattern syntax.
 * Or an object with field `context`, `selector` and optionally `strictness`.
 */
export type PatternStyle =
	| string
	| {
			/**
			 * The surrounding code that helps to resolve any ambiguity in the syntax.
			 */
			readonly context: string;
			/**
			 * The sub-syntax node kind that is the actual matcher of the pattern.
			 */
			readonly selector?: string | null;
			/**
			 * Strictness of the pattern. More strict pattern matches fewer nodes.
			 */
			readonly strictness?: Strictness | null;
	  };

/**
 * A relational rule object, which is a Rule object with two additional fields stopBy and field.
 */
export type Relation = {
	/**
	 * A list of sub rules and matches a node if all of sub rules match.
	 * The meta variables of the matched node contain all variables from the sub-rules.
	 */
	readonly all?: readonly SerializableRule[];
	/**
	 * A list of sub rules and matches a node if any of sub rules match.
	 * The meta variables of the matched node only contain those of the matched sub-rule.
	 */
	readonly any?: readonly SerializableRule[];
	readonly field?: string | null;
	/**
	 * `follows` accepts a relational rule object.
	 * the target node must appear after another node matching the `follows` sub-rule.
	 */
	readonly follows?: Relation;
	/**
	 * `has` accepts a relational rule object.
	 * the target node must has a descendant node matching the `has` sub-rule.
	 */
	readonly has?: Relation;
	/**
	 * `inside` accepts a relational rule object.
	 * the target node must appear inside of another node matching the `inside` sub-rule.
	 */
	readonly inside?: Relation;
	/**
	 * The kind name of the node to match. You can look up code's kind names in playground.
	 */
	readonly kind?: string;
	/**
	 * A utility rule id or parameterized utility call object.
	 * When multiple utility calls are present, they are combined with logical `all`.
	 */
	readonly matches?: SerializableMatches;
	/**
	 * A single sub-rule and matches a node if the sub rule does not match.
	 */
	readonly not?: SerializableRule;
	/**
	 * `nth_child` accepts number, string or object.
	 * It specifies the position in nodes' sibling list.
	 */
	readonly nthChild?: SerializableNthChild;
	/**
	 * A pattern string or a pattern object.
	 */
	readonly pattern?: PatternStyle;
	/**
	 * `precedes` accepts a relational rule object.
	 * the target node must appear before another node matching the `precedes` sub-rule.
	 */
	readonly precedes?: Relation;
	/**
	 * `range` accepts a range object.
	 * the target node must exactly appear in the range.
	 */
	readonly range?: SerializableRange;
	/**
	 * A Rust regular expression to match the node's text. https://docs.rs/regex/latest/regex/#syntax
	 */
	readonly regex?: string;
	readonly stopBy?: SerializableStopBy;
};

/**
 * Replaces a substring in the meta variable's text content with another string.
 */
export type Replace = {
	/**
	 * the replacement string
	 */
	readonly by: string;
	/**
	 * a regex to find substring to be replaced
	 */
	readonly replace: string;
	/**
	 * source meta variable to be transformed
	 */
	readonly source: string;
};

export type Rewrite = {
	readonly joinBy?: string | null;
	readonly rewriters: readonly string[];
	readonly source: string;
};

/**
 * Upstream `RuleFileGlob`, which is not exported by the pinned crate.
 */
export type RuleFileGlob =
	| string
	| {
			readonly caseInsensitive?: boolean;
			readonly glob: string;
	  };

/**
 * Separator to split string. e.g. `user_accountName` -> `user`, `accountName`
 * It will be rejoin according to `StringCase`.
 */
export type Separator = 'caseChange' | 'dash' | 'dot' | 'slash' | 'space' | 'underscore';

export type SerializableFixConfig = {
	readonly expandEnd?: Relation;
	readonly expandStart?: Relation;
	readonly template: string;
	readonly title?: string | null;
};

/**
 * A pattern string or fix object to auto fix the issue.
 * It can reference metavariables appeared in rule.
 */
export type SerializableFixer = string | SerializableFixConfig | readonly SerializableFixConfig[];

export type SerializableMatches =
	| string
	| Readonly<Record<string, Readonly<Record<string, SerializableRule>>>>;

/**
 * `nthChild` accepts either a number, a string or an object.
 */
export type SerializableNthChild =
	| NthChildSimple
	| {
			/**
			 * select the nth node that matches the rule, like CSS's of syntax
			 */
			readonly ofRule?: SerializableRule | null;
			/**
			 * nth-child syntax
			 */
			readonly position: NthChildSimple;
			/**
			 * matches from the end instead like CSS's nth-last-child
			 */
			readonly reverse?: boolean;
	  };

/**
 * Represents a zero-based character-wise position in a document
 */
export type SerializablePosition = {
	/**
	 * 0-based column number in the source code
	 */
	readonly column?: number | null;
	/**
	 * 0-based line number in the source code
	 */
	readonly line: number;
};

/**
 * Represents a position in source code using 0-based line and column numbers
 */
export type SerializableRange = {
	/**
	 * end position in the source code
	 */
	readonly end: SerializablePosition;
	/**
	 * start position in the source code
	 */
	readonly start: SerializablePosition;
};

/**
 * Used for global rules, rewriters, and pyo3/napi
 */
export type SerializableRewriter = {
	/**
	 * Additional meta variables pattern to filter matching
	 */
	readonly constraints?: Readonly<Record<string, SerializableRule>> | null;
	/**
	 * A pattern string or a FixConfig object to auto fix the issue.
	 * It can reference metavariables appeared in rule.
	 * See details in fix [object reference](https://ast-grep.github.io/reference/yaml/fix.html#fixconfig).
	 */
	readonly fix: SerializableFixer;
	/**
	 * Unique, descriptive identifier, e.g., no-unused-variable
	 */
	readonly id: string;
	/**
	 * A rule object to find matching AST nodes
	 */
	readonly rule: SerializableRule;
	/**
	 * A dictionary for metavariable manipulation. Dict key is the new variable name.
	 * Dict value is a [transformation] that specifies how meta var is processed.
	 * See [transformation doc](https://ast-grep.github.io/reference/yaml/transformation.html).
	 */
	readonly transform?: Readonly<Record<string, Transformation>> | null;
	/**
	 * Utility rules that can be used in `matches`
	 */
	readonly utils?: Readonly<Record<string, SerializableRule>> | null;
};

/**
 * A rule object to find matching AST nodes. We have three categories of rules in ast-grep.
 *
 * * Atomic: the most basic rule to match AST. We have two variants: Pattern and Kind.
 *
 * * Relational: filter matched target according to their position relative to other nodes.
 *
 * * Composite: use logic operation all/any/not to compose the above rules to larger rules.
 *
 * Every rule has it's unique name so we can combine several rules in one object.
 */
export type SerializableRule = {
	/**
	 * A list of sub rules and matches a node if all of sub rules match.
	 * The meta variables of the matched node contain all variables from the sub-rules.
	 */
	readonly all?: readonly SerializableRule[];
	/**
	 * A list of sub rules and matches a node if any of sub rules match.
	 * The meta variables of the matched node only contain those of the matched sub-rule.
	 */
	readonly any?: readonly SerializableRule[];
	/**
	 * `follows` accepts a relational rule object.
	 * the target node must appear after another node matching the `follows` sub-rule.
	 */
	readonly follows?: Relation;
	/**
	 * `has` accepts a relational rule object.
	 * the target node must has a descendant node matching the `has` sub-rule.
	 */
	readonly has?: Relation;
	/**
	 * `inside` accepts a relational rule object.
	 * the target node must appear inside of another node matching the `inside` sub-rule.
	 */
	readonly inside?: Relation;
	/**
	 * The kind name of the node to match. You can look up code's kind names in playground.
	 */
	readonly kind?: string;
	/**
	 * A utility rule id or parameterized utility call object.
	 * When multiple utility calls are present, they are combined with logical `all`.
	 */
	readonly matches?: SerializableMatches;
	/**
	 * A single sub-rule and matches a node if the sub rule does not match.
	 */
	readonly not?: SerializableRule;
	/**
	 * `nth_child` accepts number, string or object.
	 * It specifies the position in nodes' sibling list.
	 */
	readonly nthChild?: SerializableNthChild;
	/**
	 * A pattern string or a pattern object.
	 */
	readonly pattern?: PatternStyle;
	/**
	 * `precedes` accepts a relational rule object.
	 * the target node must appear before another node matching the `precedes` sub-rule.
	 */
	readonly precedes?: Relation;
	/**
	 * `range` accepts a range object.
	 * the target node must exactly appear in the range.
	 */
	readonly range?: SerializableRange;
	/**
	 * A Rust regular expression to match the node's text. https://docs.rs/regex/latest/regex/#syntax
	 */
	readonly regex?: string;
};

/**
 * Control how the relational rule search should stop
 */
export type SerializableStopBy = 'neighbor' | 'end' | SerializableRule;

export type Severity = 'off' | 'hint' | 'info' | 'warning' | 'error';

export type Strictness = 'cst' | 'smart' | 'ast' | 'relaxed' | 'signature' | 'template';

/**
 * An enumeration representing different cases for strings.
 */
export type StringCase =
	| 'lowerCase'
	| 'upperCase'
	| 'capitalize'
	| 'camelCase'
	| 'snakeCase'
	| 'kebabCase'
	| 'pascalCase';

/**
 * Extracts a substring from the meta variable's text content.
 *
 * Both `start_char` and `end_char` support negative indexing,
 * which counts character from the end of an array, moving backwards.
 */
export type Substring = {
	/**
	 * optional ending character index of the substring, defaults to the end of the string.
	 */
	readonly endChar?: number | null;
	/**
	 * source meta variable to be transformed
	 */
	readonly source: string;
	/**
	 * optional starting character index of the substring, defaults to 0.
	 */
	readonly startChar?: number | null;
};

/**
 * Represents a transformation that can be applied to a matched AST node.
 * Available transformations are `substring`, `replace` and `convert`.
 */
export type Trans =
	| {
			readonly substring: Substring;
	  }
	| {
			readonly replace: Replace;
	  }
	| {
			readonly convert: Convert;
	  }
	| {
			readonly rewrite: Rewrite;
	  };

export type Transformation = string | Trans;
