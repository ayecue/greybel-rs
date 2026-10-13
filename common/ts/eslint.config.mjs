import presets from 'presets/eslint';

/** emitter and handle-error are copies of axialcode's packages, unchanged from theirs. */
export default [{ ignores: ['emitter/', 'handle-error/'] }, ...presets];
