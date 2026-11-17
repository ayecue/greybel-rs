import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'preact/hooks';

import { GUIDE } from '../guide.js';
import type { ReaderChannel } from '../reader.generated.js';
import { useGuide } from '../state/use-guide.js';
import { useReference, type ReferenceProgress } from '../state/use-reference.js';
import { useSearch } from '../state/use-search.js';
import { useFitToView, useViewHeight } from '../state/use-view-height.js';
import { GuideSearch } from '../state/guide-search.js';
import { ALL_VISIBLE, ApiSearch } from '../state/api-search.js';
import { ChapterStructure, ChapterView } from './chapter.js';
import { ApiContents, GuideContents } from './contents.js';
import { Description } from './description.js';
import { Filter } from './filter.js';
import { ReferenceContext, getLinkedId, type ReferenceContextValue } from './reference-context.js';
import { SearchBox } from './search-box.js';
import { SectionView } from './section.js';
import { ReferenceTab, TabChoices, Tabs } from './tabs.js';

const NONE: ReadonlySet<string> = new Set();

export interface AppConfiguration extends Pick<ReferenceContextValue, 'copyLink' | 'openExample'>, ReferenceProgress {
  readonly reader: ReaderChannel;

  readonly query: string;

  readonly excludedTags: ReadonlySet<string>;

  readonly tab: ReferenceTab;

  /** The id of what to show first, once it is rendered. */
  readonly anchor?: string;

  readonly onExcludedTagsChange?: (tags: readonly string[]) => void;

  readonly onTabChange?: (tab: ReferenceTab) => void;

  readonly repositoryUrl?: string;
}

export function App(props: AppConfiguration) {
  const { reader, copyLink, openExample, onExcludedTagsChange, onTabChange } = props;
  const { overview, sections } = useReference(reader, props);
  const guideContent = useGuide(reader, GUIDE, props.onFailure);
  const structures = useMemo(
    () => guideContent.chapters.map((chapter) => new ChapterStructure(chapter)),
    [guideContent]
  );
  const [tab, setTab] = useState(props.tab);
  const [query, setQuery] = useState(props.query);
  const [guideQuery, setGuideQuery] = useState('');
  const [excludedTags, setExcludedTags] = useState(props.excludedTags);
  /** The id of what a link leads to, scrolled to once its tab, search and tags show it. */
  const [pendingId, setPendingId] = useState(props.anchor);
  /** Whether a narrow page shows its menu: the tabs and the selected tab's contents. */
  const [isMenuOpen, setMenuOpen] = useState(false);
  const result = useSearch(reader, query, excludedTags);
  const root = useRef<HTMLDivElement>(null);
  const toolbar = useRef<HTMLDivElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const currentTab = useRef(tab);
  /** Where the page was scrolled to in each tab, restored when the tab is selected again. */
  const scrollPositions = useRef(new Map<ReferenceTab, number>());
  const sectionNames = useMemo(() => new Set(overview?.contents.map((entry) => entry.name)), [overview]);
  const apiSearch = useMemo(() => (overview === undefined ? undefined : new ApiSearch(overview)), [overview]);
  const visibility = useMemo(
    () => (apiSearch === undefined || result === undefined ? ALL_VISIBLE : apiSearch.getVisibility(result)),
    [apiSearch, result]
  );

  const guideSearch = useMemo(() => new GuideSearch(structures), [structures]);
  const guideVisibility = useMemo(() => guideSearch.getVisibility(guideQuery), [guideSearch, guideQuery]);
  const isApiEmpty = overview !== undefined && visibility.hiddenTypes.size === overview.contents.length;
  const isGuideEmpty = structures.length > 0 && guideVisibility.hiddenChapters.size === structures.length;

  currentTab.current = tab;

  const changeExcludedTags = (tags: ReadonlySet<string>): void => {
    setExcludedTags(tags);
    onExcludedTagsChange?.([...tags]);
  };

  const selectTab = (next: ReferenceTab): void => {
    if (next === currentTab.current) return;

    const view = root.current?.ownerDocument.defaultView;

    if (view !== null && view !== undefined) scrollPositions.current.set(currentTab.current, view.scrollY);
    currentTab.current = next;
    setTab(next);
    onTabChange?.(next);
  };

  const reference = useMemo<ReferenceContextValue>(
    () => ({
      copyLink,
      openExample,
      hasSection: (name) => sectionNames.has(name),
      reveal: (link) => {
        const id = getLinkedId(link);

        if (id === undefined) return;
        revealId(id);
      }
    }),
    [sectionNames, copyLink, openExample]
  );

  /** Shows the element `id` names: selects its tab, clears the search and tags hiding it, and scrolls to it. */
  function revealId(id: string): void {
    const element = root.current?.ownerDocument.getElementById(id);

    if (element === null || element === undefined) return;

    const elementTab = getTab(element);
    const isFiltered = isFilteredOut(element);

    if (elementTab === currentTab.current && !isFiltered) {
      element.scrollIntoView({ block: 'start' });

      return;
    }

    if (isFiltered && elementTab === ReferenceTab.Api) {
      setQuery('');
      changeExcludedTags(NONE);
    } else if (isFiltered) {
      setGuideQuery('');
    }

    selectTab(elementTab);
    setPendingId(id);
  }

  useLayoutEffect(() => {
    if (pendingId !== undefined) return;

    const view = root.current?.ownerDocument.defaultView;
    const position = scrollPositions.current.get(tab);

    if (view === null || view === undefined) return;
    if (position !== undefined) {
      view.scrollTo({ top: position });
    } else if ((root.current?.getBoundingClientRect().top ?? 0) < 0) {
      root.current?.scrollIntoView({ block: 'start' });
    }
  }, [tab]);

  useEffect(() => {
    if (pendingId === undefined) return;

    const element = root.current?.ownerDocument.getElementById(pendingId);

    if (element === null || element === undefined) return;
    if (getTab(element) !== currentTab.current || isFilteredOut(element)) {
      revealId(pendingId);

      return;
    }

    setPendingId(undefined);
    element.scrollIntoView({ block: 'start' });
  }, [pendingId, visibility, guideVisibility, tab, structures, sections]);

  useLayoutEffect(() => {
    const bar = toolbar.current;
    const ownerDocument = bar?.ownerDocument;

    if (!isMenuOpen || bar === null || ownerDocument === undefined) return;

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      setMenuOpen(false);
    };

    const onPointerDown = (event: PointerEvent) => {
      if (bar.contains(event.target as Node)) return;
      setMenuOpen(false);
    };

    ownerDocument.addEventListener('keydown', onKeyDown);
    ownerDocument.addEventListener('pointerdown', onPointerDown);

    return () => {
      ownerDocument.removeEventListener('keydown', onKeyDown);
      ownerDocument.removeEventListener('pointerdown', onPointerDown);
    };
  }, [isMenuOpen]);

  useEffect(() => {
    const bar = toolbar.current;
    const page = root.current;

    if (bar === null || page === null) return;

    const observer = new ResizeObserver(() =>
      page.style.setProperty('--reference-toolbar-height', `${bar.getBoundingClientRect().height}px`)
    );

    observer.observe(bar);

    return () => observer.disconnect();
  }, [overview === undefined]);

  useViewHeight(root, overview !== undefined);
  useFitToView(menu, isMenuOpen);

  if (overview === undefined) return null;

  return (
    <ReferenceContext.Provider value={reference}>
      <div class="reference" ref={root}>
        <header class="overview">
          <h1>{GUIDE.title}</h1>
          {props.repositoryUrl !== undefined && (
            <a
              class="star-link"
              href={props.repositoryUrl}
              target="_blank"
              rel="noopener noreferrer"
              title="Star greybel-rs on GitHub"
            >
              <span aria-hidden="true">★</span> Star on GitHub
            </a>
          )}
        </header>
        <div class="toolbar" ref={toolbar}>
          <button
            type="button"
            class="menu-toggle"
            aria-label="Contents"
            aria-expanded={isMenuOpen}
            aria-controls="reference-menu"
            onClick={() => setMenuOpen(!isMenuOpen)}
          >
            <span aria-hidden="true">☰</span>
          </button>
          <Tabs tab={tab} onSelect={selectTab} />
          {tab === ReferenceTab.Api && (
            <Filter
              query={query}
              tags={overview.tags}
              excludedTags={excludedTags}
              onQuery={setQuery}
              onToggleTag={(tag) => {
                const tags = new Set(excludedTags);

                if (!tags.delete(tag)) tags.add(tag);
                changeExcludedTags(tags);
              }}
            />
          )}
          {tab === ReferenceTab.Guide && (
            <div class="filter">
              <SearchBox query={guideQuery} placeholder="Search the guide" onQuery={setGuideQuery} />
            </div>
          )}
          {isMenuOpen && (
            <div
              ref={menu}
              class="menu"
              id="reference-menu"
              onClick={(event) => {
                if ((event.target as Element).closest('a, .tab-choice') === null) return;
                setMenuOpen(false);
              }}
            >
              <TabChoices tab={tab} onSelect={selectTab} />
              {tab === ReferenceTab.Api ? (
                <ApiContents entries={overview.contents} visibility={visibility} />
              ) : (
                <GuideContents structures={structures} visibility={guideVisibility} />
              )}
            </div>
          )}
        </div>
        <div
          class="layout tab-panel"
          id={`panel-${ReferenceTab.Api}`}
          role="tabpanel"
          aria-labelledby={`tab-${ReferenceTab.Api}`}
          hidden={tab !== ReferenceTab.Api}
        >
          <ApiContents entries={overview.contents} visibility={visibility} />
          <div class="sections">
            {isApiEmpty && <p class="no-results">Nothing in the API matches the search.</p>}
            {sections.map((section) => (
              <SectionView
                key={section.name}
                section={section}
                isHidden={visibility.hiddenTypes.has(section.name)}
                hiddenMembers={visibility.hiddenMembers.get(section.name) ?? NONE}
              />
            ))}
          </div>
        </div>
        <div
          class="layout tab-panel"
          id={`panel-${ReferenceTab.Guide}`}
          role="tabpanel"
          aria-labelledby={`tab-${ReferenceTab.Guide}`}
          hidden={tab !== ReferenceTab.Guide}
        >
          <GuideContents structures={structures} visibility={guideVisibility} />
          <div class="sections guide">
            {isGuideEmpty && <p class="no-results">Nothing in the guide matches the search.</p>}
            {guideContent.introduction.length > 0 && (
              <section class="introduction" hidden={guideQuery.trim() !== ''}>
                <Description blocks={guideContent.introduction} />
              </section>
            )}
            {structures.map((structure) => (
              <ChapterView
                key={structure.anchor}
                structure={structure}
                isHidden={guideVisibility.hiddenChapters.has(structure.anchor)}
                hiddenParts={guideVisibility.hiddenParts}
              />
            ))}
          </div>
        </div>
      </div>
    </ReferenceContext.Provider>
  );
}

/** The tab `element` is in. */
function getTab(element: HTMLElement): ReferenceTab {
  return element.closest(`#panel-${ReferenceTab.Guide}`) === null ? ReferenceTab.Api : ReferenceTab.Guide;
}

/** Whether the search or the tags hide `element`, rather than only the tab it is in. */
function isFilteredOut(element: HTMLElement): boolean {
  return element.closest('[hidden]:not(.tab-panel)') !== null;
}
