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

/** Page guide with its manifest and chapter Markdown. */
export class Guide {
  readonly #manifest: GuideEntry;
  readonly #markdown: Readonly<Record<string, string>>;

  readonly title: string;
  readonly introduction?: string;
  readonly chapters: readonly Chapter[];

  constructor(manifest: GuideEntry, markdown: Readonly<Record<string, string>>) {
    this.#manifest = manifest;
    this.#markdown = markdown;
    this.title = this.#manifest.title;
    this.introduction =
      this.#manifest.introduction === undefined ? undefined : this.#readMarkdown(this.#manifest.introduction);
    this.chapters = this.#manifest.chapters.map((file) => ({
      anchor: this.#toAnchor(file),
      markdown: this.#readMarkdown(file)
    }));
  }

  #readMarkdown(file: string): string {
    const markdown = this.#markdown[`../guide/${file}`];

    if (markdown === undefined) throw new Error(`guide/guide.json lists ${file}, which guide/ doesn't have`);

    return markdown;
  }

  #toAnchor(file: string): string {
    return `guide-${file.slice(file.lastIndexOf('/') + 1).replace(/\.md$/, '')}`;
  }
}

const ENTRY = Object.values(
  import.meta.glob<GuideEntry>('../guide/guide.json', { eager: true, import: 'default' })
)[0]!;

const MARKDOWN = import.meta.glob<string>('../guide/**/*.md', { eager: true, query: '?raw', import: 'default' });

export const GUIDE = new Guide(ENTRY, MARKDOWN);
