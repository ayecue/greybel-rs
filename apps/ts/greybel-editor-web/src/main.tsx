import { render } from 'preact';

import { App } from './components/app.js';
import { initialProject } from './code-storage.js';
import './editor.css';

const root = document.getElementById('editor-app');

if (root === null) throw new Error('The page has no #editor-app element.');

render(<App content={await initialProject(location)} />, root);
