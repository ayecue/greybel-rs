import { useEffect, useMemo, useReducer, useRef, useState } from 'preact/hooks';

import { Builder } from '../builder.js';
import { LanguageServer } from '../editor/language-server.js';
import { ProjectEditor } from '../editor/view.js';
import { Project } from '../project.js';
import type { ProjectContent } from '../project/content.js';
import { ProjectSettings } from '../project/settings.js';
import { useServices } from './services.js';
import { ExecutePanel } from './execute-panel.js';
import { FileList } from './file-list.js';
import { WasmStatus } from './wasm-status.js';
import { TabBar } from './tab-bar.js';
import { ReferencePanel } from './reference-panel.js';
import { Splitter } from './splitter.js';
import { TransformPanel } from './transform-panel.js';

const NOTICE_TIME = 10_000;

const HINT =
  'This environment is a simulation and may not fully replicate every detail of the real in-game environment. ' +
  'To access the root account, use the username root with the password test.';

interface Notice {
  readonly id: number;
  readonly text: string;
}

export interface AppConfiguration {
  readonly content: ProjectContent;
}

export function App({ content }: AppConfiguration) {
  const editorRef = useRef<HTMLDivElement>(null);
  const breakpointChanges = useRef<((path: string, lines: number[]) => void) | undefined>(undefined);
  const services = useServices();
  const { storage } = services;
  const project = useMemo(() => new Project(content, services.paths), []);
  const [editor, setEditor] = useState<ProjectEditor>();
  const [settings, setSettings] = useState<ProjectSettings>();
  const [, refresh] = useReducer((count: number) => count + 1, 0);
  const [notices, setNotices] = useState<readonly Notice[]>([]);
  const [link, setLink] = useState<string>();
  const [isReferenceOpen, setReferenceOpen] = useState(false);
  const [hasSaveRequest, setSaveRequest] = useState(false);
  const builder = useMemo(() => new Builder(services.wasm), []);

  const notify = (text: string) => {
    const id = Date.now() + Math.random();

    setNotices((current) => [...current, { id, text }]);
    setTimeout(() => setNotices((current) => current.filter((notice) => notice.id !== id)), NOTICE_TIME);
  };

  useEffect(() => {
    const parent = editorRef.current;

    if (parent === null) return;

    const server = new LanguageServer(project, services.wasm, services.highlighter);
    const projectEditor = new ProjectEditor(parent, {
      project,
      client: server.client,
      manifestHelp: services.manifestHelp,
      onBreakpointsChange: (file, lines) => breakpointChanges.current?.(file, lines),
      onTabsChange: () => refresh(undefined)
    });
    const projectSettings = new ProjectSettings(server.client, project, services.languageServerSettings);
    const unsubscribeSettings = projectSettings.subscribe(() => projectEditor.showProblem(projectSettings.problem));
    const unsubscribe = project.subscribe((event) => {
      storage.store(project.content);
      if (event.kind === 'edit') return;
      refresh(undefined);
    });

    setEditor(projectEditor);
    setSettings(projectSettings);

    return () => {
      unsubscribe();
      unsubscribeSettings();
      projectSettings.close();
      projectEditor.destroy();
      server.close();
      builder.close();
    };
  }, []);

  const save = async () => {
    setSaveRequest(true);
    try {
      setLink(await storage.save(project.content));
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error));
    } finally {
      setSaveRequest(false);
    }
  };

  return (
    <div class="editor-app">
      <header class="toolbar">
        <h1>Greybel Editor</h1>
        <button type="button" class="share" onClick={() => setLink(storage.createShareLink(project.content))}>
          Share
        </button>
        {storage.canSave && (
          <button type="button" class="save" disabled={hasSaveRequest} onClick={() => void save()}>
            Save
          </button>
        )}
        <button
          type="button"
          class="reference-toggle"
          aria-pressed={isReferenceOpen}
          onClick={() => setReferenceOpen(!isReferenceOpen)}
        >
          API reference
        </button>
        <WasmStatus />
        <span class="hint">{HINT}</span>
      </header>
      {link !== undefined && (
        <div class="link-popup" role="dialog">
          <textarea readOnly value={link} onFocus={(event) => event.currentTarget.select()} />
          <button type="button" onClick={() => void navigator.clipboard.writeText(link)}>
            Copy
          </button>
          <button type="button" onClick={() => setLink(undefined)}>
            Close
          </button>
        </div>
      )}
      <main class="workspace">
        {editor && <FileList project={project} editor={editor} path={editor.path} onFailure={notify} />}
        {editor && <Splitter name="file-list" axis="x" side="before" min={120} />}
        <div class="code-area">
          {editor && <TabBar editor={editor} />}
          <div class="code" ref={editorRef} hidden={editor !== undefined && editor.path === undefined} />
          {editor && editor.path === undefined && <p class="no-file">Open a file of the list to edit it.</p>}
        </div>
        {isReferenceOpen && editor && <Splitter name="reference" axis="x" side="after" min={200} />}
        {isReferenceOpen && editor && <ReferencePanel editor={editor} />}
      </main>
      {editor && settings && <Splitter name="panels" axis="y" side="after" min={120} />}
      {editor && settings && (
        <div class="panels">
          <TransformPanel builder={builder} editor={editor} settings={settings} onFailure={notify} />
          <Splitter name="transform" axis="x" side="before" min={200} />
          <ExecutePanel
            editor={editor}
            project={project}
            settings={settings}
            breakpointChanges={breakpointChanges}
            onFailure={notify}
          />
        </div>
      )}
      <ul class="notices">
        {notices.map((notice) => (
          <li
            key={notice.id}
            onClick={() => setNotices((current) => current.filter((other) => other.id !== notice.id))}
          >
            {notice.text}
          </li>
        ))}
      </ul>
    </div>
  );
}
