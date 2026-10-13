import { useEffect, useMemo, useReducer, useRef, useState } from 'preact/hooks';

import { Builder } from '../builder.js';
import { canSave, saveProject, shareLink, storeProject } from '../code-storage.js';
import { startLanguageServer } from '../editor/language-server.js';
import { ProjectEditor } from '../editor/view.js';
import { Project } from '../project.js';
import type { ProjectContent } from '../project/content.js';
import { ExecutePanel } from './execute-panel.js';
import { FileList } from './file-list.js';
import { TabBar } from './tab-bar.js';
import { ReferencePanel } from './reference-panel.js';
import { TransformPanel } from './transform-panel.js';

/** How long a notice shows. */
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

/** The editor page: the project's files, the API reference, their transformation and their execution. */
export function App({ content }: AppConfiguration) {
  const editorRef = useRef<HTMLDivElement>(null);
  const breakpointChanges = useRef<((path: string, lines: number[]) => void) | undefined>(undefined);
  const project = useMemo(() => new Project(content), []);
  const [editor, setEditor] = useState<ProjectEditor>();
  const [, refresh] = useReducer((count: number) => count + 1, 0);
  const [notices, setNotices] = useState<readonly Notice[]>([]);
  const [link, setLink] = useState<string>();
  const [isReferenceOpen, setReferenceOpen] = useState(false);
  const [hasSaveRequest, setSaveRequest] = useState(false);
  const builder = useMemo(() => new Builder(), []);

  const notify = (text: string) => {
    const id = Date.now() + Math.random();

    setNotices((current) => [...current, { id, text }]);
    setTimeout(() => setNotices((current) => current.filter((notice) => notice.id !== id)), NOTICE_TIME);
  };

  useEffect(() => {
    const parent = editorRef.current;

    if (parent === null) return;

    const server = startLanguageServer(project);
    const projectEditor = new ProjectEditor(parent, {
      project,
      client: server.client,
      onBreakpointsChange: (file, lines) => breakpointChanges.current?.(file, lines),
      onTabsChange: () => refresh(undefined)
    });
    const unsubscribe = project.subscribe((event) => {
      storeProject(project.content);
      if (event.kind === 'edit') return;
      refresh(undefined);
    });

    setEditor(projectEditor);

    return () => {
      unsubscribe();
      projectEditor.destroy();
      server.close();
      builder.close();
    };
  }, []);

  const save = async () => {
    setSaveRequest(true);
    try {
      setLink(await saveProject(project.content));
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
        <button type="button" class="share" onClick={() => setLink(shareLink(location, project.content))}>
          Share
        </button>
        {canSave && (
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
        <div class="code-area">
          {editor && <TabBar editor={editor} />}
          <div class="code" ref={editorRef} hidden={editor !== undefined && editor.path === undefined} />
          {editor && editor.path === undefined && <p class="no-file">Open a file of the list to edit it.</p>}
        </div>
        {isReferenceOpen && editor && <ReferencePanel editor={editor} />}
      </main>
      {editor && (
        <div class="panels">
          <TransformPanel builder={builder} code={() => editor.code} onFailure={notify} />
          <ExecutePanel editor={editor} project={project} breakpointChanges={breakpointChanges} onFailure={notify} />
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
