/** Synthetic checked inputs and equivalent file/inline definitions; no product fixture imports. */
import filesystem from 'node:fs';
import path from 'node:path';

export function sdkInlineConfiguration(
	base: string,
	endpoint: string,
	standard = 'ACTOR_A',
): object {
	return {
		id: 'sdk-verification',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['skills/**/SKILL.md'] },
		decision_model: { model: 'jev-latest', endpoint },
		limits: { max_file_bytes: 1048576n },
		evidence: {
			kind: 'inline',
			utilities: { sdk_paragraph: { language: 'Markdown', rule: { kind: 'paragraph' } } },
			matchers: { paragraph: { language: 'Markdown', rule: { matches: 'sdk_paragraph' } } },
			kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
		},
		filters: {
			purpose: {
				per: 'skills/{name}',
				reads: { file: 'SKILL.md', kind: 'paragraph', fields: ['body'] },
				labels: ['eligible', 'blocked'],
				questions: {
					purpose: {
						type: 'choice',
						instructions: { check: 'body', task: 'Classify reusable guidance.' },
						criteria: { portable: 'Reusable guidance.', local: 'Local procedure.' },
					},
				},
				decision: {
					start: 'purpose',
					steps: {
						purpose: {
							ask: 'purpose',
							min_probability: 0.7,
							routes: { portable: { label: 'eligible' }, local: { label: 'blocked' } },
							uncertain: 'unsure',
						},
					},
				},
			},
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'sdk-standards',
				markdown: `${standard}: Each instruction names its actor. <!-- K001 -->\n`,
				entries: sdkRuleEntries(),
			},
		],
	};
}
export function sdkRuleEntries(): object {
	return {
		K001: {
			name: 'instruction_actor',
			evidence: 'paragraph',
			severity: 'warn',
			only_when: { purpose: ['eligible'] },
			questions: {
				actor: {
					type: 'noul',
					instructions: { check: 'body', against: 'standard', task: 'Is the actor named?' },
				},
			},
			decision: {
				start: 'actor',
				steps: {
					actor: {
						ask: 'actor',
						yes_at_or_above: 0.8,
						no_at_or_below: 0.2,
						yes: 'pass',
						['no']: { finding: 'Name the actor.', ['at']: 'body' },
						uncertain: 'uncertain',
					},
				},
			},
		},
	};
}
export function createCheckedSdkInputs(root: string, sentinel = 'CHECKED_A'): void {
	filesystem.mkdirSync(path.join(root, 'skills', 'portable'), { recursive: true });
	filesystem.writeFileSync(
		path.join(root, 'skills', 'portable', 'SKILL.md'),
		`${sentinel}: The caller names its actor.\n`,
	);
}
export function createSdkFileDefinitions(root: string, endpoint: string): void {
	filesystem.mkdirSync(path.join(root, 'evidence', 'rules'), { recursive: true });
	filesystem.mkdirSync(path.join(root, 'evidence', 'utils'), { recursive: true });
	filesystem.writeFileSync(
		path.join(root, 'evidence', 'sgconfig.yml'),
		'ruleDirs: [rules]\nutilDirs: [utils]\n',
	);
	filesystem.writeFileSync(
		path.join(root, 'evidence', 'utils', 'paragraph.yml'),
		'id: sdk_paragraph\nlanguage: Markdown\nrule: {kind: paragraph}\n',
	);
	filesystem.writeFileSync(
		path.join(root, 'evidence', 'rules', 'paragraph.yml'),
		'id: paragraph\nlanguage: Markdown\nrule: {matches: sdk_paragraph}\n',
	);
	filesystem.writeFileSync(
		path.join(root, 'rulebook.md'),
		'ACTOR_A: Each instruction names its actor. <!-- K001 -->\n',
	);
	filesystem.writeFileSync(path.join(root, 'rules.json'), JSON.stringify(sdkRuleEntries()));
	const inline = sdkInlineConfiguration(root, endpoint);
	// These fields are authored independently in the CLI vocabulary; no hidden SDK file conversion.
	filesystem.writeFileSync(
		path.join(root, 'sunstack.lint.json'),
		JSON.stringify({
			marker_prefix: 'K',
			check: { files: ['skills/**/SKILL.md'] },
			decision_model: { model: 'jev-latest', endpoint },
			limits: { max_file_bytes: 1048576 },
			rulebook: [{ markdown: 'rulebook.md', rules: 'rules.json' }],
			evidence: {
				ast_grep: 'evidence/sgconfig.yml',
				kinds: { paragraph: { rule: 'paragraph', fields: { body: { from: 'remainder' } } } },
			},
			...('filters' in inline ? { filters: inline.filters } : {}),
		}),
	);
}
export function createBlockedSdkInput(root: string): void {
	filesystem.mkdirSync(path.join(root, 'skills', 'local'), { recursive: true });
	filesystem.writeFileSync(
		path.join(root, 'skills', 'local', 'SKILL.md'),
		'SDK_LOCAL_ONLY: One local instruction.\n',
	);
}

/** Independent source literals and expected policy effects, from main21's contract. */
export const sdkDocumentPage =
	'<h2>Parent</h2><ex-markdown><h3>Island</h3></ex-markdown><custom-hidden><h3>Custom</h3></custom-hidden><template><h3>Template</h3></template><ex-mermaid label="Legacy">Legacy body</ex-mermaid><custom-diagram CAPTION="Generic" alt="Picture" aria-label="Access">Diagram body</custom-diagram><h2>Last</h2><p>Tail</p>';
export const sdkDocumentLinks = [
	['heading', 'target.md#heading'],
	['missing', 'target.md#missing'],
	['explicit', 'target.md#explicit'],
	['named', 'target.md#named'],
	['html', 'target.html#html-id'],
	['html_missing', 'target.html#absent'],
	['text', 'notes.txt#part'],
	['file', 'target.md'],
	['absent', 'absent.md#part'],
	['outside', '../../outside.md#part'],
] as const;
export const sdkDocumentLinkText = sdkDocumentLinks
	.map(([name, target]) => `[${name}](${target})`)
	.join('\n\n');
const standardDocument = {
	html: { skip_tags: [], text_attributes: {} },
	markdown: { heading_anchors: null },
} as const;
const restoredDocument = {
	html: { skip_tags: ['ex-markdown'], text_attributes: { 'ex-mermaid': ['label'] } },
	markdown: { heading_anchors: 'github' },
} as const;
const customDocument = {
	html: { skip_tags: ['custom-hidden'], text_attributes: { 'CUSTOM-DIAGRAM': ['CAPTION', 'ALT'] } },
	markdown: { heading_anchors: 'github' },
} as const;
const plainAsking =
	'Parent Island Custom Template Legacy body [alt: Picture] [aria-label: Access] Diagram body Last Tail';
const restoredAsking =
	'Parent Island Custom Template [label: Legacy] Legacy body [alt: Picture] [aria-label: Access] Diagram body Last Tail';
const customAsking =
	'Parent Island Custom Template Legacy body [caption: Generic] [alt: Picture] [aria-label: Access] Diagram body Last Tail';
export const sdkDocumentCases = {
	standard: {
		settings: standardDocument,
		headings: ['Parent', 'Island', 'Custom', 'Last'],
		asking: plainAsking,
		github: false,
	},
	restored: {
		settings: restoredDocument,
		headings: ['Parent', 'Custom', 'Last'],
		asking: restoredAsking,
		github: true,
	},
	custom: {
		settings: customDocument,
		headings: ['Parent', 'Island', 'Last'],
		asking: customAsking,
		github: true,
	},
	skipOnly: {
		settings: {
			...restoredDocument,
			html: { ...restoredDocument.html, skip_tags: ['custom-hidden'] },
		},
		headings: ['Parent', 'Island', 'Last'],
		asking: restoredAsking,
		github: true,
	},
	attributesOnly: {
		settings: {
			...restoredDocument,
			html: { ...restoredDocument.html, text_attributes: customDocument.html.text_attributes },
		},
		headings: ['Parent', 'Custom', 'Last'],
		asking: customAsking,
		github: true,
	},
	anchorsOnly: {
		settings: { ...restoredDocument, markdown: standardDocument.markdown },
		headings: ['Parent', 'Custom', 'Last'],
		asking: restoredAsking,
		github: false,
	},
} as const;
export type SdkDocumentCase = keyof typeof sdkDocumentCases;

export const sdkDocumentKinds = {
	page: { rule: 'html_page', fields: { body: { from: 'first_line' } } },
	outline: {
		html_units: { mode: 'unit', units: ['section'] },
		fields: { raw: { from: 'section', text: 'full' } },
	},
	link: {
		rule: 'markdown_link',
		fields: {
			targets: {
				from: 'target',
				regex: '\\[[^\\]]+\\]\\(([^)]+)\\)',
				resolve: 'path',
				opening_chars: 120,
			},
		},
	},
} as const;
const documentRuleEntries = {
	K001: {
		name: 'document_words',
		evidence: 'page',
		severity: 'warn',
		questions: {
			actor: { type: 'noul', instructions: { check: 'body', task: 'Is the document complete?' } },
		},
		decision: {
			start: 'actor',
			steps: {
				actor: {
					ask: 'actor',
					yes_at_or_above: 0.8,
					no_at_or_below: 0.2,
					yes: 'pass',
					['no']: { finding: 'Incomplete words.' },
					uncertain: 'uncertain',
				},
			},
		},
	},
	K002: {
		name: 'document_outline',
		evidence: 'outline',
		severity: 'warn',
		decision: {
			start: 'raw',
			steps: {
				raw: {
					mechanical_check: { tool: 'contains', field: 'raw', text: '<h' },
					yes: 'pass',
					['no']: { finding: 'Raw heading missing.' },
				},
			},
		},
	},
	K003: {
		name: 'document_links',
		evidence: 'link',
		severity: 'warn',
		decision: {
			start: 'anchor',
			steps: {
				anchor: {
					mechanical_check: { tool: 'anchor_exists', field: 'targets' },
					yes: 'pass',
					['no']: { finding: 'Anchor missing.' },
				},
			},
		},
	},
} as const;
const documentStandard = 'Document settings are explicit.';
const documentRulebook = `${documentStandard} <!-- K001 -->\n\nOutline stays raw. <!-- K002 -->\n\nLinks keep reasons. <!-- K003 -->\n`;

/** The expected root value is independently authored from settings_value's approved fields. */
export function sdkDocumentRootValue(endpoint: string, selected: SdkDocumentCase): object {
	return {
		marker_prefix: 'K',
		max_question_words: 400,
		check: { files: ['docs/page.html', 'docs/links.md'], ignore: [] },
		decision_model: { model: 'jev-latest', endpoint },
		limits: null,
		filters: {},
		kinds: sdkDocumentKinds,
		document: sdkDocumentCases[selected].settings,
	};
}
export function sdkDocumentRequest(selected: SdkDocumentCase): object {
	return {
		model: 'jev-latest',
		questions: {
			actor: { type: 'noul', instructions: { check: 'body', task: 'Is the document complete?' } },
		},
		state: {
			body: sdkDocumentCases[selected].asking,
			standard: documentStandard,
			'unavailable-fields': {},
		},
	};
}
export function sdkDocumentConfiguration(
	base: string,
	endpoint: string,
	selected: SdkDocumentCase,
	project = false,
): object {
	return {
		id: 'sdk-document-settings',
		base_directory: base,
		marker_prefix: 'K',
		check: { files: ['docs/page.html', 'docs/links.md'] },
		decision_model: { model: 'jev-latest', endpoint },
		evidence: {
			kind: project ? 'project' : 'inline',
			...(project
				? { config_path: 'evidence/sgconfig.yml' }
				: {
						matchers: {
							html_page: { language: 'Html', rule: { kind: 'document' } },
							markdown_link: { language: 'Markdown', rule: { kind: 'paragraph' } },
						},
						utilities: {},
					}),
			kinds: sdkDocumentKinds,
			...sdkDocumentCases[selected].settings,
		},
		rulebooks: [
			{
				kind: 'inline',
				id: 'document-settings-rules',
				markdown: documentRulebook,
				entries: documentRuleEntries,
			},
		],
	};
}
export function sdkDocumentFileConfiguration(endpoint: string, selected: SdkDocumentCase): object {
	return {
		marker_prefix: 'K',
		check: { files: ['docs/page.html', 'docs/links.md'] },
		decision_model: { model: 'jev-latest', endpoint },
		evidence: {
			ast_grep: 'evidence/sgconfig.yml',
			kinds: sdkDocumentKinds,
			...sdkDocumentCases[selected].settings,
		},
		rulebook: [{ markdown: 'document-rulebook.md', rules: 'document-rules.json' }],
	};
}
export function createSdkDocumentInputs(root: string, definitions?: string): void {
	for (const [file, content] of Object.entries({
		'docs/page.html': sdkDocumentPage,
		'docs/links.md': sdkDocumentLinkText,
		'docs/target.md': '# Heading\n\n<a id="explicit"></a>\n<a name="named"></a>\n',
		'docs/target.html': '<p id="html-id">Target</p>',
		'docs/notes.txt': 'part\n',
		'evidence/sgconfig.yml': 'ruleDirs: [rules]\n',
		'evidence/rules/html.yml': 'id: html_page\nlanguage: Html\nrule: {kind: document}\n',
		'evidence/rules/markdown.yml':
			'id: markdown_link\nlanguage: Markdown\nrule: {kind: paragraph}\n',
		'document-rulebook.md': documentRulebook,
		'document-rules.json': JSON.stringify(documentRuleEntries),
	})) {
		const destination = file.startsWith('docs/') ? root : definitions;
		if (destination === undefined) continue;
		filesystem.mkdirSync(path.dirname(path.join(destination, file)), { recursive: true });
		filesystem.writeFileSync(path.join(destination, file), content);
	}
}
