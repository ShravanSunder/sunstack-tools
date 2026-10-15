export function escapeHtml(sourceText: string): string {
	return sourceText
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll('"', '&quot;')
		.replaceAll("'", '&#39;');
}

export function decodeEscapedHtmlText(sourceText: string): string {
	return sourceText
		.replaceAll('&lt;', '<')
		.replaceAll('&gt;', '>')
		.replaceAll('&quot;', '"')
		.replaceAll('&#39;', "'")
		.replaceAll('&amp;', '&');
}

export function htmlDocument(title: string, content: string): string {
	return `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>${escapeHtml(title)}</title><style>
  :root{font-family:system-ui,sans-serif;color:#142433;background:white;line-height:1.65}body{margin:0}main{max-width:1080px;margin:auto;padding:32px 24px 60px}h1{font-size:42px;line-height:1.15;letter-spacing:-.035em}h2{margin-top:40px}a{color:#087978}table{border-collapse:collapse;width:100%}td,th{padding:14px;border:1px solid #d7e2e8;text-align:left}figure{margin:24px 0;overflow:auto;padding:16px;border:1px solid #d7e2e8}figure>svg{display:block;width:100%;height:680px;max-width:100%;margin:auto}aside{border-left:4px solid #087978;background:#eef5f6;padding:16px 24px;margin:24px 0}.risk{border-color:#ab501a;background:#fff1e8}.timeline{border-left:2px solid #9abec9;padding:8px 24px}.graph-controls{display:flex;gap:10px;flex-wrap:wrap}button{background:#eaf2f5;border:1px solid #9abec9;padding:10px 20px;font:inherit;cursor:pointer}button[aria-pressed=true]{background:#142433;color:white}.graph-panel[hidden]{display:none}.change-list{display:grid;gap:12px}pre{white-space:pre-wrap;overflow-wrap:anywhere;font-size:13px;background:#eef3f6;padding:20px}details{margin-block:24px}summary{cursor:pointer;font-weight:650}
  </style></head><body><main>${content}</main></body></html>`;
}
