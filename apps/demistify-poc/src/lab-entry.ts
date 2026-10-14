import reportSource from '../references/deep-research-report.md?raw';

const sourceReportLink = document.querySelector<HTMLAnchorElement>('#source-report-link');
const sourceReportDetails = document.querySelector<HTMLDetailsElement>('#source-report');
const sourceReportText = document.querySelector<HTMLPreElement>('#source-report-text');
if (!sourceReportLink || !sourceReportDetails || !sourceReportText) {
	throw new Error('The lab landing page is missing its source report view.');
}
sourceReportText.textContent = reportSource;
sourceReportLink.addEventListener('click', (): void => {
	sourceReportDetails.open = true;
});
