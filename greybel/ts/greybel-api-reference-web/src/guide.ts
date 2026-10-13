/** Guide manifest from `guide/guide.json`, listing paths relative to `guide/`. */
interface GuideEntry {
  readonly title: string;
  readonly introduction?: string;
  readonly chapters: readonly string[];
}

export interface Chapter {
  /** Chapter anchor ID: `guide-` plus the filename used by its links. */
  readonly anchor: string;
  readonly markdown: string;
}

export interface Guide {
  readonly title: string;
  readonly introduction?: string;
  readonly chapters: readonly Chapter[];
}

const ENTRY = Object.values(
  import.meta.glob<GuideEntry>('../guide/guide.json', { eager: true, import: 'default' })
)[0]!;

const MARKDOWN = import.meta.glob<string>('../guide/**/*.md', { eager: true, query: '?raw', import: 'default' });

/** Page guide containing its title, introduction Markdown, and chapters. */
export const GUIDE: Guide = {
  title: ENTRY.title,
  introduction: ENTRY.introduction === undefined ? undefined : markdownOf(ENTRY.introduction),
  chapters: ENTRY.chapters.map((file) => ({ anchor: anchorOf(file), markdown: markdownOf(file) }))
};

function markdownOf(file: string): string {
  const markdown = MARKDOWN[`../guide/${file}`];

  if (markdown === undefined) throw new Error(`guide/guide.json lists ${file}, which guide/ doesn't have`);

  return markdown;
}

function anchorOf(file: string): string {
  return `guide-${file.slice(file.lastIndexOf('/') + 1).replace(/\.md$/, '')}`;
}
