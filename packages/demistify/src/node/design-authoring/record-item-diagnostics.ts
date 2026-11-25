import type { ZodError } from 'zod';

export const releasedRecordMigration =
	'From a Demistify source checkout at the 1.33.0 release commit a9e9a34c, run: node packages/demistify/scripts/migrate-lean-design-records.ts --released-lean --source <copied-docs> --project <relative-project> --dry-run; then run packages/demistify/scripts/initiative-migration at the 1.36.1 release commit da3363b6 (its README gives the command).';

const guidance = new Map<string, string>([
	['id', 'give the record an id starting with a letter. Use letters, numbers, and hyphens.'],
	[
		'kind',
		"give the record a kind. Use 'choice' or 'ruling' for a decision and 'assumption' for an assumption.",
	],
	['statement', 'write a statement that says what was decided or what is being assumed.'],
	['question', 'a choice needs a question. Write the question the options answer.'],
	['options', 'a choice needs at least two options. Add two options, or write this as a ruling.'],
	[
		'options.*.id',
		'give the option an id starting with a letter. Use letters, numbers, and hyphens.',
	],
	['options.*.title', 'give the option a title that says what it does.'],
	[
		'recommended',
		"say whether an option is recommended. Set recommended to 'none' or give the option id and the agent who recommends it.",
	],
	['authority', 'a decision needs authority. Record who can answer or who decided.'],
	[
		'authority.state',
		"set authority.state to 'owner-open', 'owner-decided', or 'agent-decided'. Only a choice can be open.",
	],
	[
		'authority.chosen',
		'a settled choice needs a chosen option. Set chosen to one of its option ids.',
	],
	['authority.decision', 'a settled choice needs decision text. Write what was decided.'],
	['authority.decidedOn', 'a settled decision needs a real date. Set decidedOn to YYYY-MM-DD.'],
	[
		'authority.sourceKind',
		'name the basis for the decision. Use owner-words or owner-accepted for an owner decision, or author or contract for an agent decision.',
	],
	[
		'authority.source',
		'this decision needs a source. Add the relative link and an excerpt of the owner’s words or contract.',
	],
	[
		'authority.source.href',
		'link to the owner’s words or contract. Add a relative href to source.',
	],
	[
		'authority.source.excerpt',
		'quote the relevant owner wording or contract passage in source.excerpt.',
	],
	['authority.by', 'an agent’s decision needs an author. Add by with the agent alias and role.'],
	['authority.by.alias', 'name the agent who made the decision in by.alias.'],
	['authority.by.role', 'set by.role to lead, implementer, verifier, reviewer, or advisor.'],
	[
		'authority.rationale',
		'an agent’s decision needs a rationale. Explain why the agent made this call.',
	],
	[
		'authority.weight',
		"an agent's decision needs a weight. Add weight: 'design', 'process' or 'minor'.",
	],
	['by', 'an assumption needs an author. Set by to an object with alias and role.'],
	['weight', "an assumption needs a weight. Add weight: 'design', 'process' or 'minor'."],
	['status', "set the assumption’s status to 'open', 'confirmed', or 'refuted'."],
	[
		'settles',
		'an open assumption needs settles: one sentence saying what observation would settle it.',
	],
	[
		'resolution',
		'a confirmed or refuted assumption needs resolution evidence. Add the relative link and an excerpt of what was observed.',
	],
	['resolution.href', 'link to the resolution evidence. Add a relative href to resolution.'],
	[
		'resolution.excerpt',
		'quote the observation that settled the assumption in resolution.excerpt.',
	],
	[
		'settledBy',
		'a confirmed or refuted assumption needs settledBy. Give the alias of the person or agent who settled it.',
	],
]);

export function recordItemValidationError(file: string, input: unknown, error: ZodError): Error {
	const records: readonly unknown[] = Array.isArray(input) ? input : [];
	const messages = error.issues.map(({ path, message }): string => {
		const [index, ...fields] = path;
		if (typeof index !== 'number') return `${file}: export an array of records.`;
		const record = records[index];
		const name =
			typeof record === 'object' && record && 'id' in record && typeof record.id === 'string'
				? record.id
				: `Record ${index + 1}`;
		if (typeof record === 'object' && record && 'authority' in record) {
			const { authority } = record;
			if (
				typeof authority === 'object' &&
				authority &&
				('owner' in authority ||
					'lead' in authority ||
					'question' in authority ||
					('state' in authority && authority.state === 'lead-decided'))
			)
				return `${file}: ${name}: migrate the 1.32.1 record. Put the question and options on the choice. Use agent-decided with by: { alias, role } and weight for an agent decision. ${releasedRecordMigration}`;
		}
		if (
			typeof record === 'object' &&
			record &&
			'status' in record &&
			!('by' in record) &&
			!('weight' in record)
		)
			return `${file}: ${name}: an assumption needs by: { alias, role } and weight. Migrating from 1.32.1? ${releasedRecordMigration}`;
		const key = fields.map((field) => (typeof field === 'number' ? '*' : String(field))).join('.');
		let fix = guidance.get(key);
		if (
			fields[0] === 'options' &&
			typeof record === 'object' &&
			record &&
			'options' in record &&
			Array.isArray(record.options) &&
			record.options.length < 2
		) {
			const count = record.options.length === 1 ? 'one' : 'none';
			fix = `a choice needs at least two options and has ${count}. Add an option, or write this as a ruling.`;
		}
		if (!fix && fields[0] === 'recommended') fix = guidance.get('recommended');
		return `${file}: ${name}: ${fix ?? `${fields.join('.') || 'record'}: ${message}`}`;
	});
	return new Error([...new Set(messages)].join('\n'));
}
