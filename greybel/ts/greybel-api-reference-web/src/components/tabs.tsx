/** The reference's two tabs: the API's types and members, and the scripting guide. */
export enum ReferenceTab {
  Api = 'api',
  Guide = 'guide'
}

const TITLES: Record<ReferenceTab, string> = {
  [ReferenceTab.Api]: 'API reference',
  [ReferenceTab.Guide]: 'Guide'
};

export function Tabs({ tab, onSelect }: { tab: ReferenceTab; onSelect: (tab: ReferenceTab) => void }) {
  return (
    <div class="tabs" role="tablist">
      {Object.values(ReferenceTab).map((each) => (
        <button
          type="button"
          role="tab"
          class="tab"
          id={`tab-${each}`}
          aria-selected={each === tab}
          aria-controls={`panel-${each}`}
          onClick={() => onSelect(each)}
        >
          {TITLES[each]}
        </button>
      ))}
    </div>
  );
}

/** The tabs as the menu of a narrow page lists them, above the selected tab's contents. */
export function TabChoices({ tab, onSelect }: { tab: ReferenceTab; onSelect: (tab: ReferenceTab) => void }) {
  return (
    <div class="tab-choices">
      {Object.values(ReferenceTab).map((each) => (
        <button
          type="button"
          class="tab-choice"
          aria-current={each === tab ? 'page' : undefined}
          onClick={() => onSelect(each)}
        >
          {TITLES[each]}
        </button>
      ))}
    </div>
  );
}
