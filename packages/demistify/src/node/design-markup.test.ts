import { expect, test } from 'vitest';

import { checkHtml } from './check.ts';
import { collectDesignRecords } from './design-markup.ts';

const rule = {
	kind: 'specification',
	id: 'Rule',
	title: 'Keep input',
	needs: ['Need'],
	bodyRef: 'rule-body',
	summaryRef: 'rule-summary',
};
function markup(parts: string): string {
	return `<script type="application/json" data-ex-records>${JSON.stringify([rule])}</script><template id="rule-body"><p id="rule-summary">Keep input after refusal.</p>${parts}</template>`;
}

test('explicit rule and cases survive a separate trailing paragraph', (): void => {
	const source = markup(
		'<p data-ex-part="rule">Observable conditions.</p><figure data-ex-part="cases"><figcaption>Refusal.</figcaption></figure><p>Trailing note.</p>',
	);
	expect(collectDesignRecords(source, 'specification.html')[0]?.bodyHtml).toContain(
		'data-ex-part="rule">Observable conditions.',
	);
});

test.each([
	['<p data-ex-part="rule">First.</p><p data-ex-part="rule">Second.</p>', 'duplicate part rule'],
	['<p data-ex-part="procedure">Steps.</p>', 'forbidden part procedure'],
	['<p data-ex-part="unknown">Other.</p>', 'part unknown'],
])('the checker shares authored part rejection: %s', async (parts, message): Promise<void> => {
	const findings = await checkHtml(markup(parts), { diagrams: false });
	expect(
		findings.some(
			(finding) => finding.rule === 'design-collection' && finding.message.includes(message),
		),
	).toBe(true);
});

test.each(['<ex-call-stack id="calls"></ex-call-stack>'])(
	'the checker diagnoses unprepared public reader input: %s',
	async (reader): Promise<void> => {
		const findings = await checkHtml(reader, { diagrams: false });
		expect(
			findings.some(
				(finding) =>
					finding.rule === 'design-collection' && finding.message.includes('demistify prepare'),
			),
		).toBe(true);
	},
);
