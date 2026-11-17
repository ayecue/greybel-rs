import { ReferenceTab, openApiReference } from 'greybel-api-reference-web';
import wasm from 'greybel-api-reference-web/greybel-api-reference-web.wasm?url';

/** Local-storage key for reference tags excluded between visits. */
const EXCLUDED_TAGS = 'excludedTags';

const address = new URL(window.location.href);
const excludedTagsJson = window.localStorage.getItem(EXCLUDED_TAGS);

openApiReference(document.getElementById('reference')!, {
  wasm,
  documentationUrl: new URL(address.pathname, address.origin).toString(),
  query: address.searchParams.get('filter') ?? undefined,
  excludedTags: excludedTagsJson === null ? undefined : (JSON.parse(excludedTagsJson) as string[]),
  onExcludedTagsChange: (tags) => window.localStorage.setItem(EXCLUDED_TAGS, JSON.stringify(tags)),
  tab: address.searchParams.get('tab') === ReferenceTab.Guide ? ReferenceTab.Guide : ReferenceTab.Api,
  anchor: address.hash === '' ? undefined : decodeURIComponent(address.hash.slice(1)),
  onTabChange: showTab,
  repositoryUrl: 'https://github.com/ayecue/greybel-rs'
});

/** Stores the selected tab in the URL: `?tab=guide` for the guide; no parameter for the default API tab. */
function showTab(tab: ReferenceTab): void {
  const url = new URL(window.location.href);

  if (tab === ReferenceTab.Guide) url.searchParams.set('tab', tab);
  else url.searchParams.delete('tab');
  window.history.replaceState(null, '', url);
}
