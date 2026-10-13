import { describe, expect, it, onTestFinished, vi } from 'vitest';
import { ChannelError, ErrorCode, Transport } from 'duplex-core';

import { MessageTargetDriver, WorkerHost, WorkerScope } from '../src/index.js';
import { Echo, EchoPeer, PageChannel, Shout } from './support/echo.page.generated.js';
import { WorkerChannel } from './support/echo.worker.generated.js';
import {
  CLOSE,
  echoHost,
  Echoer,
  fakeWorker,
  parseFrames,
  handedPort,
  NO_CHECKS,
  OfferedEchoer
} from './support/echo.js';
import * as offeredPage from './support/offered-echo.page.generated.js';
import * as offeredWorker from './support/offered-echo.worker.generated.js';

describe('WorkerHost', () => {
  it('should connect to the worker, messages pendingReply until it listens', async () => {
    const worker = fakeWorker();
    const { echo } = echoHost(worker).channels;
    const reply = echo.send(new Shout({ text: 'hi' }));

    new WorkerChannel((await WorkerScope.accept(worker.scope)).transport, NO_CHECKS).use(new Echoer());
    await expect(reply).resolves.toBe('hi');
    expect(echo.connection).toBeDefined();
  });

  it('should offer a channel the worker connects to', async () => {
    const worker = fakeWorker();
    const host = new WorkerHost(worker, {
      echo: { channelClass: offeredPage.PageChannel, options: NO_CHECKS }
    });
    const reply = host.channels.echo.send(new offeredPage.Shout({ text: 'hi' }));

    new offeredWorker.WorkerChannel((await WorkerScope.accept(worker.scope)).transport, NO_CHECKS).use(
      new OfferedEchoer()
    );
    await expect(reply).resolves.toBe('hi');
  });

  it('should relay a channel of another transport to the worker over a port of its own', async () => {
    const worker = fakeWorker();
    const { port1, port2 } = new MessageChannel();
    const remote = new PageChannel(new MessageTargetDriver(port1), NO_CHECKS);
    const host = new WorkerHost(
      worker,
      { offered: { channelClass: offeredPage.PageChannel, options: NO_CHECKS } },
      { relays: [{ transport: new Transport(new MessageTargetDriver(port2)), protocol: Echo, peer: EchoPeer.Worker }] }
    );
    const scope = await WorkerScope.accept(worker.scope);

    onTestFinished(() => {
      port1.close();
      port2.close();
    });
    new WorkerChannel(scope.transportOf(Echo), NO_CHECKS).use(new Echoer());
    new offeredWorker.WorkerChannel(scope.transportOf(offeredWorker.OfferedEcho), NO_CHECKS).use(new OfferedEchoer());

    await expect(remote.send(new Shout({ text: 'relayed' }))).resolves.toBe('relayed');
    await expect(host.channels.offered.send(new offeredPage.Shout({ text: 'own' }))).resolves.toBe('own');
    expect(scope.transportOf(Echo)).not.toBe(scope.transport);
    expect(worker.posted[0]).toEqual(Uint32Array.of(Echo.channelId));
  });

  it('should open a channel on a port another worker opened, the other worker listening for it', async () => {
    const server = fakeWorker();
    const serverHost = new WorkerHost(server, {});
    const scopes: WorkerScope[] = [];
    const stop = WorkerScope.listen(server.scope, (scope) => {
      new WorkerChannel(scope.transport, NO_CHECKS).use(new Echoer());
      scopes.push(scope);
    });
    const client = fakeWorker();

    onTestFinished(stop);
    new WorkerHost(client, {}, { ports: [{ protocol: Echo, port: serverHost.open() }] });

    const scope = await WorkerScope.accept(client.scope);
    const port = scope.portTransportOf(Echo);
    const channel = new PageChannel(port!, NO_CHECKS);

    await expect(channel.send(new Shout({ text: 'over a port' }))).resolves.toBe('over a port');
    expect(scope.portTransportOf(offeredWorker.OfferedEcho)).toBeUndefined();
    expect(client.posted[0]).toEqual(Uint32Array.of(Echo.channelId));
    expect(scopes).toHaveLength(2);
  });

  it('should close a relayed channel for its remote when the worker fails', async () => {
    const worker = fakeWorker();
    const { port1, port2 } = new MessageChannel();
    const remote = new PageChannel(new MessageTargetDriver(port1), NO_CHECKS);

    new WorkerHost(
      worker,
      {},
      { relays: [{ transport: new Transport(new MessageTargetDriver(port2)), protocol: Echo, peer: EchoPeer.Worker }] }
    );

    const scope = await WorkerScope.accept(worker.scope);

    onTestFinished(() => {
      port1.close();
      port2.close();
    });
    new WorkerChannel(scope.transportOf(Echo), NO_CHECKS).use(new Echoer());
    await remote.send(new Shout({ text: 'relayed' }));
    worker.fail('Uncaught RuntimeError: unreachable');

    await expect(remote.closeEvent).resolves.toBe(ErrorCode.Panic);
  });

  it('should close the channels when the worker throws', async () => {
    const worker = fakeWorker();
    const host = echoHost(worker);
    const reply = host.channels.echo.send(new Shout({ text: 'hi' }));

    worker.fail('Uncaught SyntaxError');
    await expect(reply).rejects.toEqual(ChannelError.channelEnd(ErrorCode.Panic));
    await expect(host.closeEvent).resolves.toBe(ErrorCode.Panic);
    await expect(host.channels.echo.closeEvent).resolves.toBe(ErrorCode.Panic);
    expect(worker.listenerCount).toBe(0);
  });

  it('should fail a request the worker does not route', async () => {
    const worker = fakeWorker();
    const { echo } = echoHost(worker).channels;

    new WorkerChannel((await WorkerScope.accept(worker.scope)).transport, NO_CHECKS);

    const reply = echo.send(new Shout({ text: 'hi' }));

    await expect(reply).rejects.toEqual(ChannelError.reject(ErrorCode.UnhandledMessage));
  });

  it('should close a channel the worker closed', async () => {
    const worker = fakeWorker();
    const { echo } = echoHost(worker).channels;
    const workerChannel = new WorkerChannel((await WorkerScope.accept(worker.scope)).transport, NO_CHECKS);

    await echo.send(new Shout({ text: 'hi' })).catch(() => undefined);
    workerChannel.close(ErrorCode.Panic);
    await expect(echo.closeEvent).resolves.toBe(ErrorCode.Panic);
  });

  it('should terminate the worker and close its channels', async () => {
    const worker = fakeWorker();
    const host = echoHost(worker);
    const { echo } = host.channels;
    const port = await handedPort(worker.scope);
    const receivedBatches: Uint8Array[] = [];

    port.addEventListener('message', ({ data }) => receivedBatches.push(data as Uint8Array));
    new WorkerChannel(new MessageTargetDriver(port), NO_CHECKS).use(new Echoer());
    await echo.send(new Shout({ text: 'hi' }));

    const pendingReply = echo.send(new Shout({ text: 'again' }));

    host.close();
    host.close();
    expect(worker.terminated).toBe(true);
    await expect(pendingReply).rejects.toEqual(ChannelError.channelEnd(ErrorCode.ClosedChannel));
    await vi.waitFor(() => expect(parseFrames(receivedBatches.at(-1)!).at(-1)?.kind).toBe(CLOSE));
    expect([...(parseFrames(receivedBatches.at(-1)!).at(-1)?.payload ?? [])]).toEqual([ErrorCode.ClosedChannel]);
  });

  it('should hand the worker a port of its own, apart from what else is posted to it', async () => {
    const worker = fakeWorker();
    const { echo } = echoHost(worker).channels;

    new WorkerChannel((await WorkerScope.accept(worker.scope)).transport, NO_CHECKS).use(new Echoer());
    worker.postMessage(Uint8Array.of(0xff, 0xff, 0xff));

    const reply = echo.send(new Shout({ text: 'hi' }));

    await expect(reply).resolves.toBe('hi');
    expect(worker.posted[0]).toEqual(new Uint32Array());
  });

  it('should type its channels by their declaration', () => {
    const host = echoHost(fakeWorker());

    onTestFinished(() => host.close());

    // @ts-expect-error: no channel is declared as `echoes`.
    const echoes = host.channels.echoes;

    expect(echoes).toBeUndefined();
  });

  it('should reject routers that do not fit their channel', () => {
    const worker = fakeWorker();
    const declare = () =>
      new WorkerHost(worker, {
        // @ts-expect-error: the worker's router doesn't route the page's channel.
        echo: { channelClass: PageChannel, routers: [new Echoer()] }
      });

    expect(declare).toBeTypeOf('function');
  });
});
