import { describe, expect, it } from 'vitest';

import { Output, TextScreen } from '../src/index.js';

describe('TextScreen', () => {
  it('should replace a full line', () => {
    const screen = new TextScreen();

    screen.print(new Output.Print({ text: '10%', newLine: true, replace: false }));
    screen.print(new Output.Print({ text: '20%', newLine: true, replace: true }));
    expect(screen.lines).toEqual(['20%', '']);
  });

  it('should replace an open line in place', () => {
    const screen = new TextScreen();

    screen.print(new Output.Print({ text: 'loading', newLine: false, replace: false }));
    screen.print(new Output.Print({ text: 'done', newLine: true, replace: true }));
    expect(screen.lines).toEqual(['done', '']);
  });

  it('should only print when replacing on a blank screen', () => {
    const screen = new TextScreen();
    const outputs = [
      new Output.Print({ text: 'a', newLine: true, replace: false }),
      new Output.Clear(),
      new Output.Print({ text: 'b', newLine: true, replace: true })
    ];

    screen.apply(outputs);
    expect(screen.text).toBe('b\n');
  });

  it('should replace only the last of several lines', () => {
    const screen = new TextScreen();

    screen.print(new Output.Print({ text: 'a\nb', newLine: true, replace: false }));
    screen.print(new Output.Print({ text: 'c', newLine: true, replace: true }));
    expect(screen.lines).toEqual(['a', 'c', '']);
  });
});
