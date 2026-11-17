import { useState } from 'preact/hooks';

import type { ProjectEditor } from '../editor/view.js';
import type { Project } from '../project.js';
import { MANIFEST_PATH } from '../project/content.js';

export interface FileListConfiguration {
  readonly project: Project;
  readonly editor: ProjectEditor;
  readonly path: string | undefined;
  readonly onFailure: (message: string) => void;
}

type NameEdit = { readonly kind: 'creation' } | { readonly kind: 'renaming'; readonly path: string };

export function FileList({ project, editor, path, onFailure }: FileListConfiguration) {
  const [nameEdit, setNameEdit] = useState<NameEdit>();
  const [name, setName] = useState('');

  const startEdit = (edit: NameEdit) => {
    setNameEdit(edit);
    setName(edit.kind === 'renaming' ? edit.path : '');
  };

  const confirmEdit = () => {
    if (nameEdit === undefined) return;

    const target = project.paths.normalize(name);
    const problem = nameEdit.kind === 'creation' ? project.create(target) : editor.rename(nameEdit.path, target);

    if (problem !== undefined) {
      onFailure(problem);

      return;
    }

    setNameEdit(undefined);
    if (nameEdit.kind !== 'creation') return;
    editor.open(target);
  };

  const removeFile = (file: string) => {
    if (!confirm(`Delete ${file}?`)) return;

    const problem = editor.delete(file);

    if (problem === undefined) return;
    onFailure(problem);
  };

  const nameField = (
    <input
      type="text"
      class="file-name"
      value={name}
      placeholder="lib/util.src"
      ref={(input) => input?.focus()}
      onInput={(event) => setName(event.currentTarget.value)}
      onKeyDown={(event) => {
        if (event.key === 'Enter') confirmEdit();
        if (event.key !== 'Escape') return;
        setNameEdit(undefined);
      }}
      onBlur={() => setNameEdit(undefined)}
    />
  );

  return (
    <nav class="file-list" aria-label="Files">
      <header class="file-list-header">
        <h2>Files</h2>
        <button type="button" class="new-file" title="Add a file" onClick={() => startEdit({ kind: 'creation' })}>
          +
        </button>
      </header>
      <ul>
        <li
          key={MANIFEST_PATH}
          class={`manifest${MANIFEST_PATH === path ? ' active' : ''}${project.has(MANIFEST_PATH) ? '' : ' template'}`}
          data-path={MANIFEST_PATH}
        >
          <span
            class="file-path"
            onClick={() => editor.open(MANIFEST_PATH)}
            title="The project's settings for runs, transforms and diagnostics"
          >
            {MANIFEST_PATH}
          </span>
        </li>
        {project.scripts.map((file) =>
          nameEdit?.kind === 'renaming' && nameEdit.path === file ? (
            <li key={file}>{nameField}</li>
          ) : (
            <li key={file} class={file === path ? 'file active' : 'file'} data-path={file}>
              <button
                type="button"
                class={file === project.entry ? 'entry-marker entry' : 'entry-marker'}
                title={file === project.entry ? 'New terminals run this file' : 'Run this file in new terminals'}
                aria-pressed={file === project.entry}
                onClick={() => project.setEntry(file)}
              >
                ▶
              </button>
              <span
                class="file-path"
                onClick={() => editor.open(file)}
                onDblClick={() => startEdit({ kind: 'renaming', path: file })}
                title="Double-click to rename"
              >
                {file}
              </span>
              <button type="button" class="delete-file" title={`Delete ${file}`} onClick={() => removeFile(file)}>
                ×
              </button>
            </li>
          )
        )}
        {nameEdit?.kind === 'creation' && <li>{nameField}</li>}
      </ul>
    </nav>
  );
}
