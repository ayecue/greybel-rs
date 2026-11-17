import { indentWithTab } from '@codemirror/commands';
import { indentUnit } from '@codemirror/language';
import { formatKeymap, type LSPClient } from '@codemirror/lsp-client';
import { EditorState, type Extension } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { basicSetup } from 'codemirror';

import type { Project } from '../project.js';
import { MANIFEST_PATH } from '../project/content.js';
import { MANIFEST_TEMPLATE } from '../project/manifest.js';
import type { ManifestProblem } from '../project/manifest-problem.js';
import type { PathResolver } from '../project/paths.js';
import { BreakpointGutter } from './breakpoints.js';
import type { ManifestHelp } from './manifest-help.js';
import { SemanticTokenHighlighter } from './semantic-tokens.js';
import { editorHighlightStyle, editorTheme } from './theme.js';

export interface ProjectEditorOptions {
  readonly project: Project;
  readonly client: LSPClient;
  readonly manifestHelp: ManifestHelp;
  readonly onBreakpointsChange: (path: string, lines: number[]) => void;
  readonly onTabsChange: () => void;
}

export interface StopPlace {
  readonly path: string;
  readonly line: number;
}

/** Retains each file's undo history, selection and breakpoints after its tab closes. */
export class ProjectEditor {
  readonly view: EditorView;
  readonly #options: ProjectEditorOptions;
  readonly #states = new Map<string, EditorState>();
  readonly #breakpoints = new BreakpointGutter();
  readonly #semanticTokens = new SemanticTokenHighlighter();
  readonly #extensions: Extension;
  readonly #emptyState = EditorState.create({ extensions: [EditorState.readOnly.of(true), editorTheme] });

  #tabs: string[];
  #path: string | undefined;
  #stop: StopPlace | undefined;
  #problem: ManifestProblem | undefined;

  constructor(parent: HTMLElement, options: ProjectEditorOptions) {
    this.#options = options;
    this.#extensions = [
      basicSetup,
      indentUnit.of('\t'),
      keymap.of([indentWithTab, ...formatKeymap]),
      editorTheme,
      editorHighlightStyle
    ];
    this.#path = options.project.entry;
    this.#tabs = [this.#path];
    this.view = new EditorView({ parent, state: this.#createState(this.#path) });
  }

  get #paths(): PathResolver {
    return this.#options.project.paths;
  }

  get tabs(): readonly string[] {
    return this.#tabs;
  }

  get path(): string | undefined {
    return this.#path;
  }

  get code(): string | undefined {
    return this.#path === undefined ? undefined : this.view.state.doc.toString();
  }

  open(path: string): void {
    if (path === this.#path || (!this.#options.project.has(path) && path !== MANIFEST_PATH)) return;
    if (!this.#tabs.includes(path)) {
      const selectedIndex = this.#path === undefined ? this.#tabs.length - 1 : this.#tabs.indexOf(this.#path);

      this.#tabs = [...this.#tabs.slice(0, selectedIndex + 1), path, ...this.#tabs.slice(selectedIndex + 1)];
    }

    this.#show(path);
  }

  close(path: string): void {
    const index = this.#tabs.indexOf(path);

    if (index < 0) return;
    this.#tabs = this.#tabs.filter((tab) => tab !== path);
    if (path === this.#path) {
      this.#show(this.#tabs[index] ?? this.#tabs[index - 1]);

      return;
    }

    this.#options.onTabsChange();
  }

  /** Returns an error message on failure. */
  rename(from: string, to: string): string | undefined {
    const problem = this.#options.project.rename(from, to);

    if (problem !== undefined || from === to) return problem;
    this.#tabs = this.#tabs.map((tab) => (tab === from ? to : tab));
    if (from === this.#path) {
      const lines = this.#breakpoints.getLines(this.view.state);

      this.#path = to;
      this.view.setState(this.#breakpoints.restore(this.#createState(to), lines));
    } else {
      const state = this.#states.get(from);

      this.#states.delete(from);
      if (state !== undefined)
        this.#states.set(to, this.#breakpoints.restore(this.#createState(to), this.#breakpoints.getLines(state)));
    }

    if (this.#stop?.path === from) this.#stop = { ...this.#stop, path: to };
    this.#showStop();
    this.#options.onTabsChange();

    return undefined;
  }

  /** Returns an error message on failure. */
  delete(path: string): string | undefined {
    const problem = this.#options.project.delete(path);

    if (problem !== undefined) return problem;
    this.#states.delete(path);
    if (path !== this.#path) {
      this.close(path);

      return undefined;
    }

    const index = this.#tabs.indexOf(path);

    this.#tabs = this.#tabs.filter((tab) => tab !== path);
    this.#path = undefined;
    this.#show(this.#tabs[index] ?? this.#tabs[index - 1]);

    return undefined;
  }

  /** One-based lines keyed by file URI. */
  getBreakpoints(): Map<string, number[]> {
    const states = new Map(this.#states);

    if (this.#path !== undefined) states.set(this.#path, this.view.state);

    return new Map(
      [...states]
        .filter(([path]) => !this.#paths.isManifest(path))
        .map(([path, state]): [string, number[]] => [this.#paths.toUri(path), this.#breakpoints.getLines(state)])
    );
  }

  showProblem(problem: ManifestProblem | undefined): void {
    this.#problem = problem;
    this.#showProblem();
  }

  showStop(stop: StopPlace | undefined): void {
    this.#stop = stop;
    if (stop !== undefined && stop.path !== this.#path) this.open(stop.path);
    this.#showStop();
  }

  /** Opens the entry file when no file is selected. */
  insertCode(code: string): void {
    if (this.#path === undefined || this.#paths.isManifest(this.#path)) this.open(this.#options.project.entry);

    const { from, to } = this.view.state.selection.main;
    const line = this.view.state.doc.lineAt(from);
    const prefix = from === line.from ? '' : '\n';

    this.view.dispatch({
      changes: { from, to, insert: `${prefix}${code}\n` },
      scrollIntoView: true
    });
    this.view.focus();
  }

  destroy(): void {
    this.view.destroy();
  }

  #show(path: string | undefined): void {
    if (this.#path !== undefined && this.#path !== path) this.#states.set(this.#path, this.view.state);
    this.#path = path;
    if (path === undefined) {
      this.view.setState(this.#emptyState);
    } else {
      this.view.setState(this.#states.get(path) ?? this.#createState(path));
      this.#states.delete(path);
    }

    this.#showStop();
    this.#showProblem();
    this.#options.onTabsChange();
  }

  #showStop(): void {
    if (this.#path === undefined || this.#paths.isManifest(this.#path)) return;
    this.#breakpoints.showStopLine(this.view, this.#stop?.path === this.#path ? this.#stop.line : undefined);
  }

  #showProblem(): void {
    if (this.#path === undefined || !this.#paths.isManifest(this.#path)) return;
    this.view.dispatch(
      this.#options.manifestHelp.createProblemUpdate(
        this.view.state,
        this.#problem?.path === this.#path ? this.#problem : undefined
      )
    );
  }

  #createState(path: string): EditorState {
    const { project, client, onBreakpointsChange } = this.#options;

    if (this.#paths.isManifest(path)) {
      return EditorState.create({
        doc: project.read(path) ?? MANIFEST_TEMPLATE,
        extensions: [
          basicSetup,
          keymap.of([indentWithTab]),
          editorTheme,
          editorHighlightStyle,
          this.#options.manifestHelp.extension(),
          EditorView.updateListener.of((update) => {
            if (!update.docChanged) return;

            const code = update.state.doc.toString();

            if (project.has(path)) project.write(path, code);
            else project.create(path, code);
          })
        ]
      });
    }

    return EditorState.create({
      doc: project.read(path) ?? '',
      extensions: [
        this.#extensions,
        client.plugin(this.#paths.toUri(path), 'greyscript'),
        this.#semanticTokens.extension(),
        this.#breakpoints.extension((lines) => onBreakpointsChange(path, lines)),
        EditorView.updateListener.of((update) => {
          if (!update.docChanged) return;
          project.write(path, update.state.doc.toString());
        })
      ]
    });
  }
}
