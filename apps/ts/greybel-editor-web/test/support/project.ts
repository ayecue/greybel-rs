import { EditorPage } from './editor-page.js';

export const UTIL = 'greet = function(name)\n  return "hi " + name\nend function';

export const FILES = { 'main.src': 'print(1)', 'a.src': 'print("a")', 'b.src': 'print("b")' };

/** `params[0]` "write" writes the shared file; "wait" reads a line, then prints the file. */
const WORLD_PROGRAM = [
  'computer = get_shell.host_computer',
  'if params.len > 0 and params[0] == "write" then',
  '  computer.touch(home_dir, "shared.txt")',
  '  computer.File(home_dir + "/shared.txt").set_content("from the writer")',
  '  print("written")',
  '  exit',
  'end if',
  'if params.len > 0 and params[0] == "wait" then user_input("Read? ")',
  'file = computer.File(home_dir + "/shared.txt")',
  'if file then print("content: " + file.get_content) else print("content: none")'
].join('\n');

/** Profiles `write` and `wait` set those params. */
export const WORLD_PROJECT = EditorPage.encodeProject('main.src', {
  'main.src': WORLD_PROGRAM,
  'greybel.json': JSON.stringify({
    profiles: { write: { run: { params: ['write'] } }, wait: { run: { params: ['wait'] } } }
  })
});
