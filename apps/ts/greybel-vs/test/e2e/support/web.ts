import 'mocha/mocha';

export async function run(): Promise<void> {
  mocha.setup({ ui: 'bdd', reporter: undefined });
  await import('../language-client.test');
  await import('../preview.test');
  await import('../api-reference.test');
  await import('../debug.test');
  await import('../build.test');
  await import('../project.test');
  await import('../terminal.test');
  await import('../world.test');
  await new Promise<void>((resolve, reject) => {
    mocha.run((failures) => (failures > 0 ? reject(new Error(`${failures} tests failed`)) : resolve()));
  });
}
