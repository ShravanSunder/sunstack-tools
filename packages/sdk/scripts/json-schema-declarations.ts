// Render the pinned ast-grep transport JSON schema as readonly TypeScript types.
//
// Objects are closed (no index signature), so object-literal checking rejects
// misspelled properties. Only the JSON Schema forms schemars emits are accepted;
// anything else stops generation instead of widening to `unknown`.

type JsonSchema = boolean | Readonly<Record<string, unknown>>;

function isSchemaObject(value: unknown): value is Readonly<Record<string, unknown>> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function asSchema(value: unknown, where: string): JsonSchema {
	if (typeof value === 'boolean' || isSchemaObject(value)) return value;
	throw new Error(`${where}: not a JSON schema`);
}

function schemaList(value: unknown, where: string): JsonSchema[] {
	if (!Array.isArray(value)) throw new Error(`${where}: expected a schema list`);
	return value.map((item: unknown, index) => asSchema(item, `${where}[${index}]`));
}

const identifier = /^[A-Za-z_$][A-Za-z0-9_$]*$/u;

function propertyName(name: string): string {
	return identifier.test(name) ? name : JSON.stringify(name);
}

function comment(schema: Readonly<Record<string, unknown>>, indent: string): string {
	const description = schema['description'];
	if (typeof description !== 'string' || description.trim() === '') return '';
	const lines = description
		.trim()
		.split('\n')
		.map((line) => `${indent} * ${line.replaceAll('*/', '*\\/')}`.trimEnd());
	return `${indent}/**\n${lines.join('\n')}\n${indent} */\n`;
}

function literal(value: unknown, where: string): string {
	if (
		value === null ||
		typeof value === 'string' ||
		typeof value === 'number' ||
		typeof value === 'boolean'
	) {
		return JSON.stringify(value);
	}
	throw new Error(`${where}: unsupported literal`);
}

function referenceName(reference: string, where: string): string {
	const match = /^#\/\$defs\/(.+)$/u.exec(reference);
	if (match?.[1] === undefined) throw new Error(`${where}: unsupported $ref ${reference}`);
	return match[1];
}

function typeKeyword(
	kind: unknown,
	schema: Readonly<Record<string, unknown>>,
	indent: string,
	where: string,
): string {
	switch (kind) {
		case 'null':
			return 'null';
		case 'boolean':
			return 'boolean';
		case 'string':
			return 'string';
		case 'number':
			return 'number';
		case 'integer':
			// Marked integers are full-width and cross as exact bigint.
			return schema['x-sunstack-full-width'] === true ? 'bigint' : 'number';
		case 'array': {
			if (schema['prefixItems'] !== undefined) {
				const elements = schemaList(schema['prefixItems'], `${where}.prefixItems`).map(
					(item, index) => render(item, indent, `${where}.prefixItems[${index}]`),
				);
				return `readonly [${elements.join(', ')}]`;
			}
			const items =
				schema['items'] === undefined ? true : asSchema(schema['items'], `${where}.items`);
			return `readonly (${render(items, indent, `${where}.items`)})[]`;
		}
		case 'object':
			return renderObject(schema, indent, where);
		default:
			throw new Error(`${where}: unsupported type ${String(kind)}`);
	}
}

function renderObject(
	schema: Readonly<Record<string, unknown>>,
	indent: string,
	where: string,
): string {
	const properties = schema['properties'];
	const required = new Set(
		Array.isArray(schema['required'])
			? schema['required'].filter((name) => typeof name === 'string')
			: [],
	);
	const additional = schema['additionalProperties'];
	const members: string[] = [];
	if (properties !== undefined) {
		if (!isSchemaObject(properties)) throw new Error(`${where}.properties: expected an object`);
		for (const [name, propertySchema] of Object.entries(properties)) {
			const property = asSchema(propertySchema, `${where}.properties.${name}`);
			const doc = isSchemaObject(property) ? comment(property, `${indent}\t`) : '';
			const optional = required.has(name) ? '' : '?';
			members.push(
				`${doc}${indent}\treadonly ${propertyName(name)}${optional}: ${render(property, `${indent}\t`, `${where}.properties.${name}`)};`,
			);
		}
	}
	const closed = members.length === 0 ? '{}' : `{\n${members.join('\n')}\n${indent}}`;
	if (additional === undefined || additional === false) {
		return members.length === 0 && additional === undefined
			? 'Readonly<Record<string, never>>'
			: closed;
	}
	const valueType =
		additional === true
			? 'JsonData'
			: render(
					asSchema(additional, `${where}.additionalProperties`),
					indent,
					`${where}.additionalProperties`,
				);
	// An index-signature literal (not Record) lets recursive aliases such as JsonData resolve.
	const record = `{ readonly [key: string]: ${valueType} }`;
	return members.length === 0 ? record : `${closed} & ${record}`;
}

function render(schema: JsonSchema, indent: string, where: string): string {
	if (schema === true) return 'JsonData';
	if (schema === false) return 'never';
	if (typeof schema['$ref'] === 'string') return referenceName(schema['$ref'], where);
	if (schema['const'] !== undefined) return literal(schema['const'], where);
	if (Array.isArray(schema['enum'])) {
		return schema['enum']
			.map((value: unknown, index) => literal(value, `${where}.enum[${index}]`))
			.join(' | ');
	}
	for (const key of ['oneOf', 'anyOf'] as const) {
		if (schema[key] !== undefined) {
			const options = schemaList(schema[key], `${where}.${key}`).map((option, index) =>
				render(option, indent, `${where}.${key}[${index}]`),
			);
			const base =
				schema['type'] === undefined && schema['properties'] === undefined
					? ''
					: `${renderBase(schema, indent, where)} & `;
			return `${base}(${options.join(' | ')})`;
		}
	}
	if (schema['allOf'] !== undefined) {
		const members = schemaList(schema['allOf'], `${where}.allOf`).map((member, index) =>
			render(member, indent, `${where}.allOf[${index}]`),
		);
		const base =
			schema['type'] === undefined && schema['properties'] === undefined
				? []
				: [renderBase(schema, indent, where)];
		return [...base, ...members].map((member) => `(${member})`).join(' & ');
	}
	return renderBase(schema, indent, where);
}

function renderBase(
	schema: Readonly<Record<string, unknown>>,
	indent: string,
	where: string,
): string {
	const kind = schema['type'];
	if (Array.isArray(kind))
		return kind.map((item: unknown) => typeKeyword(item, schema, indent, where)).join(' | ');
	if (kind === undefined) {
		if (schema['properties'] !== undefined) return renderObject(schema, indent, where);
		return 'JsonData';
	}
	return typeKeyword(kind, schema, indent, where);
}

/** Declarations for every `$defs` entry, plus the recursive JSON data type. */
export function renderSchemaDeclarations(schemaValue: unknown): string {
	if (!isSchemaObject(schemaValue)) throw new Error('schema root must be an object');
	const definitions = schemaValue['$defs'];
	if (!isSchemaObject(definitions)) throw new Error('schema has no $defs');
	const declarations = Object.entries(definitions)
		.toSorted(([left], [right]) => left.localeCompare(right))
		.map(([name, definition]) => {
			const schema = asSchema(definition, `$defs.${name}`);
			const doc = isSchemaObject(schema) ? comment(schema, '') : '';
			return `${doc}export type ${name} = ${render(schema, '', `$defs.${name}`)};`;
		});
	return [
		'// Generated from the pinned ast-grep 0.51.0 transport schema. Do not edit.',
		'',
		...declarations.flatMap((declaration) => [declaration, '']),
	].join('\n');
}
