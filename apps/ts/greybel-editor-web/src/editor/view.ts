import { indentWithTab } from '@codemirror/commands';
import { indentUnit } from '@codemirror/language';
import { formatKeymap, type LSPClient } from '@codemirror/lsp-client';
import { EditorState, type Extension } from '@codemirror/state';
import { EditorView, keymap } from '@codemirror/view';
import { basicSetup } from 'codemirror';

import type { Project } from '../project.js';
import { fileUri } from '../project/paths.js';
import { breakpointLines, breakpoints, showStopLine, withBreakpoints } from './breakpoints.js';
import { semanticTokenMarks } from './semantic-tokens.js';
import { editorTheme } from './theme.js';

export interface ProjectEditorOptions {
  readonly project: Project;
  readonly client: LSPClient;
  /** Reports that breakpoints for `path` changed to one-based `lines`. */
  readonly onBreakpointsChange: (path: string, lines: number[]) => void;
  /** Reports a tab or selected-file change. */
  readonly onTabsChange: () => void;
}

/** Program stop location with a file and one-based line. */
export interface StopPlace {
  readonly path: string;
  readonly line: number;
}

/**
 * Project file editor with one tab per open file and one view for the selected file. Retains each file's undo
 * history, selection, and breakpoints after its tab closes. The selected file's language-client plugin opens
 * it on the server; other files are read from the project.
 */
export class ProjectEditor {
  readonly view: EditorView;
  readonly #options: ProjectEditorOptions;
  readonly #states = new Map<string, EditorState>();
  readonly #extensions: Extension;
  /** Editor state when no file is selected. */
  readonly #emptyState = EditorState.create({ extensions: [EditorState.readOnly.of(true), editorTheme] });

  #tabs: string[];
  #path: string | undefined;
  #stop: StopPlace | undefined;

  constructor(parent: HTMLElement, options: ProjectEditorOptions) {
    this.#options = options;
    this.#extensions = [basicSetup, indentUnit.of('\t'), keymap.of([indentWithTab, ...formatKeymap]), editorTheme];
    this.#path = options.project.entry;
    this.#tabs = [this.#path];
    this.view = new EditorView({ parent, state: this.#createState(this.#path) });
  }

  /** Open file paths in tab order. */
  get tabs(): readonly string[] {
    return this.#tabs;
  }

  /** Selected file path, or undefined when no tabs are open. */
  get path(): string | undefined {
    return this.#path;
  }

  /** Selected file contents, or undefined when no tabs are open. */
  get code(): string | undefined {
    return this.#path === undefined ? undefined : this.view.state.doc.toString();
  }

  /** Opens `path`, adding a tab after the selected tab if it is not already open. */
  open(path: string): void {
    if (path === this.#path || !this.#options.project.has(path)) return;
    if (!this.#tabs.includes(path)) {
      const selectedIndex = this.#path === undefined ? this.#tabs.length - 1 : this.#tabs.indexOf(this.#path);

      this.#tabs = [...this.#tabs.slice(0, selectedIndex + 1), path, ...this.#tabs.slice(selectedIndex + 1)];
    }

    this.#show(path);
  }

  /** Closes the tab for `path` and selects an adjacent tab if it was selected. */
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

  /** Moves `from` to `to`, including its tab and breakpoints; returns an error on failure. */
  rename(from: string, to: string): string | undefined {
    const problem = this.#options.project.rename(from, to);

    if (problem !== undefined || from === to) return problem;
    this.#tabs = this.#tabs.map((tab) => (tab === from ? to : tab));
    if (from === this.#path) {
      const lines = breakpointLines(this.view.state);

      this.#path = to;
      this.view.setState(withBreakpoints(this.#createState(to), lines));
    } else {
      const state = this.#states.get(from);

      this.#states.delete(from);
      if (state !== undefined) this.#states.set(to, withBreakpoints(this.#createState(to), breakpointLines(state)));
    }

    if (this.#stop?.path === from) this.#stop = { ...this.#stop, path: to };
    this.#showStop();
    this.#options.onTabsChange();

    return undefined;
  }

  /** Deletes `path` and closes its tab; returns an error on failure. */
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

  /** One-based breakpoint lines keyed by file URI. */
  breakpoints(): Map<string, number[]> {
    const states = new Map(this.#states);

    if (this.#path !== undefined) states.set(this.#path, this.view.state);

    return new Map([...states].map(([path, state]): [string, number[]] => [fileUri(path), breakpointLines(state)]));
  }

  /** Opens and marks the program stop location, or clears the marker while the program runs. */
  showStop(stop: StopPlace | undefined): void {
    this.#stop = stop;
    if (stop !== undefined && stop.path !== this.#path) this.open(stop.path);
    this.#showStop();
  }

  /**
   * Inserts `code` on separate lines at the selected file's selection; opens the entry file if none is
   * selected.
   */
  insertCode(code: string): void {
    if (this.#path === undefined) this.open(this.#options.project.entry);

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

  /** Display the file at `path`, or display no file when `path` is undefined. */
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
    this.#options.onTabsChange();
  }

  #showStop(): void {
    if (this.#path === undefined) return;
    showStopLine(this.view, this.#stop?.path === this.#path ? this.#stop.line : undefined);
  }

  #createState(path: string): EditorState {
    const { project, client, onBreakpointsChange } = this.#options;

    return EditorState.create({
      doc: project.read(path) ?? '',
      extensions: [
        this.#extensions,
        client.plugin(fileUri(path), 'greyscript'),
        semanticTokenMarks(),
        breakpoints((lines) => onBreakpointsChange(path, lines)),
        EditorView.updateListener.of((update) => {
          if (!update.docChanged) return;
          project.write(path, update.state.doc.toString());
        })
      ]
    });
  }
}
