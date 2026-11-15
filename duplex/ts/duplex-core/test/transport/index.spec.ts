import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { ConnectionStateKind } from '../../src/channel/connection/state.js';
import { NO_CONNECTION } from '../../src/const.js';
import { SliceKeyKind } from '../../src/frame/route.js';
import { Bundle, EntropyUnavailable, ErrorCode, Transport, Role, Security } from '../../src/index.js';
import { SecurityRole } from '../../src/security/handshake.js';
import * as client from '../calculator.client.generated.js';
import * as provider from '../calculator.provider.generated.js';
import { Credentials, Session } from '../login.generated.js';
import * as member from '../login.member.generated.js';
import * as service from '../login.service.generated.js';
import {
  ack,
  Adder,
  MemoryDriver,
  NO_CHECKS,
  PortDriver,
  restoreMocksAndTimers,
  settle,
  settled,
  zeroRandomValues
} from '../support/channel.js';
import { TestAliases } from '../support/frames.js';
import { Doorman } from '../support/login.js';

beforeEach(zeroRandomValues);

afterEach(restoreMocksAndTimers);

describe('a transport', () => {
  it('should write what its channels send in one task as one batch', async () => {
    const driver = new MemoryDriver();
    const user = new client.UserChannel(driver, NO_CHECKS);

    new member.MemberChannel(driver, new Credentials({ user: 'ada', token: 't0k' }), NO_CHECKS);
    await settled();

    const posted = driver.posted.splice(0);

    expect(posted).toHaveLength(1);

    const slices = new TestAliases().read(posted[0]);

    expect(slices.map((slice) => slice.channel)).toEqual([client.Calculation.channelId, member.Login.channelId]);
    expect(user.connection).toBeUndefined();
  });

  it('should write a batch for each bundle', async () => {
    const driver = new MemoryDriver();

    new client.UserChannel(driver, { ...NO_CHECKS, bundle: Bundle.key(7) });
    new member.MemberChannel(driver, new Credentials({ user: 'ada', token: 't0k' }), NO_CHECKS);
    await settled();

    const posted = driver.posted.splice(0);
    const aliases = new TestAliases();

    expect(posted.map((batch) => aliases.read(batch).map((slice) => slice.channel))).toEqual([
      [client.Calculation.channelId],
      [member.Login.channelId]
    ]);
  });

  it('should send a message in the bundle it is sent with, after what its channel sent before', async () => {
    const driver = new MemoryDriver();
    const user = new client.UserChannel(driver, NO_CHECKS);

    await settled();
    driver.deliver(Role.Client, NO_CONNECTION, ack());
    driver.posted.splice(0);
    user.send(new client.Log({ text: 'first' }));
    user.send(new client.Log({ text: 'second' }), { bundle: Bundle.key(1) });
    user.send(new client.Log({ text: 'third' }));
    await settled();

    const aliases = new TestAliases();
    const frames = driver.posted
      .splice(0)
      .map((batch) => aliases.read(batch).reduce((count, slice) => count + slice.frames.length, 0));

    expect(frames).toEqual([1, 1, 1]);
  });

  it('should announce an alias once, then use it', async () => {
    const driver = new MemoryDriver();
    const user = new client.UserChannel(driver, NO_CHECKS);

    await settle();
    driver.posted.splice(0);
    driver.deliver(Role.Client, NO_CONNECTION, ack());
    user.send(new client.Log({ text: 'first' }));
    await settle();
    user.send(new client.Log({ text: 'second' }));
    await settle();

    const aliases = new TestAliases();
    const keys = driver.posted.flatMap((batch) => aliases.readKeyed(batch).map(({ key }) => key.kind));

    expect(keys).toEqual([SliceKeyKind.Announcement, SliceKeyKind.Alias]);
  });

  it('should carry the channels of two protocols over one transport', async () => {
    const ports = new MessageChannel();
    const near = new Transport(new PortDriver(ports.port1));
    const far = new Transport(new PortDriver(ports.port2));
    const user = new client.UserChannel(near, NO_CHECKS);
    const signedIn = new member.MemberChannel(near, new Credentials({ user: 'ada', token: 't0k' }), NO_CHECKS);
    const calculator = new provider.CalculatorChannel(far, NO_CHECKS).use(new Adder());
    const doorman = new service.ServiceChannel(far, NO_CHECKS).use(new Doorman());
    const [sum, name] = await Promise.all([
      user.send(new client.Add({ a: 2n, b: 3n })),
      signedIn.send(new member.Whoami())
    ]);

    expect(sum).toBe(5n);
    expect(name).toBe('ada');
    expect(
      signedIn.state.kind === ConnectionStateKind.Active ? (signedIn.state.ack as Session).greeting : undefined
    ).toBe('hello, ada');
    for (const channel of [user, signedIn, calculator, doorman]) channel.close();
    ports.port1.close();
    ports.port2.close();
  });

  it('should refuse a second channel of the same peer of a protocol', () => {
    const driver = new MemoryDriver();

    new client.UserChannel(driver, NO_CHECKS);

    const second = () => new client.UserChannel(driver, NO_CHECKS);

    expect(second).toThrow(TypeError);
  });

  it('should serve from a channel spawned behind a port', async () => {
    const ports = new MessageChannel();
    const near = new Transport(new PortDriver(ports.port1));
    const far = new Transport(new PortDriver(ports.port2));
    const user = new client.UserChannel(near, NO_CHECKS);
    const { port } = far.relay(provider.Calculation, provider.CalculationPeer.Calculator);
    const calculator = new provider.CalculatorChannel(new PortDriver(port), NO_CHECKS).use(new Adder());
    const sum = await user.send(new client.Add({ a: 2n, b: 3n }));

    expect(sum).toBe(5n);
    user.close();
    calculator.close();
    await settled();
    port.close();
    ports.port1.close();
    ports.port2.close();
  });

  it('should close a relayed connection on both sides with the reason the relay closes with', async () => {
    const ports = new MessageChannel();
    const near = new Transport(new PortDriver(ports.port1));
    const far = new Transport(new PortDriver(ports.port2));
    const user = new client.UserChannel(near, NO_CHECKS);
    const relay = far.relay(provider.Calculation, provider.CalculationPeer.Calculator);
    const calculator = new provider.CalculatorChannel(new PortDriver(relay.port), NO_CHECKS).use(new Adder());
    const ends: ErrorCode[] = [];

    calculator.onClose(({ reason }) => ends.push(reason));
    await user.send(new client.Add({ a: 2n, b: 3n }));
    relay.close(ErrorCode.Panic);
    await settled();

    await expect(user.closeEvent).resolves.toBe(ErrorCode.Panic);
    expect(ends).toEqual([ErrorCode.Panic]);
    calculator.close();
    relay.port.close();
    ports.port1.close();
    ports.port2.close();
  });

  it('should close a relayed connection for the relayed channel when the transport disconnects', async () => {
    const ports = new MessageChannel();
    const near = new Transport(new PortDriver(ports.port1));
    const farDriver = new PortDriver(ports.port2);
    const far = new Transport(farDriver);
    const relay = far.relay(client.Calculation, client.CalculationPeer.User);
    const relayed = new client.UserChannel(new PortDriver(relay.port), NO_CHECKS);

    new provider.CalculatorChannel(near, NO_CHECKS).use(new Adder());
    await relayed.send(new client.Add({ a: 2n, b: 3n }));
    farDriver.disconnect();

    await expect(relayed.closeEvent).resolves.toBe(ErrorCode.ConnectionLoss);
    relay.port.close();
    ports.port1.close();
    ports.port2.close();
  });
});

describe('credit', () => {
  it('should give each channel its share of a batch', async () => {
    const ports = new MessageChannel();
    const nearDriver = new PortDriver(ports.port1);
    const sent: Uint8Array[] = [];
    const send = nearDriver.send.bind(nearDriver);

    nearDriver.send = (batch) => {
      sent.push(batch.slice());
      send(batch);
    };

    const near = new Transport(nearDriver);
    const far = new Transport(new PortDriver(ports.port2));
    const user = new client.UserChannel(near, NO_CHECKS);
    const signedIn = new member.MemberChannel(near, new Credentials({ user: 'ada', token: 't0k' }), NO_CHECKS);

    new provider.CalculatorChannel(far, NO_CHECKS).use(new Adder());
    new service.ServiceChannel(far, NO_CHECKS).use(new Doorman());
    await Promise.all([signedIn.send(new member.Whoami()), user.send(new client.Add({ a: 0n, b: 0n }))]);

    const before = sent.length;
    const long = 'x'.repeat(30_000);

    void signedIn.send(new member.Whoami());
    for (let log = 0; log < 3; log++) user.send(new client.Log({ text: long }));
    expect(sent.length - before).toBe(1);
    await settle();

    const aliases = new TestAliases();
    const read = sent.map((batch) => aliases.read(batch).map((slice) => [slice.channel, slice.frames.length]));

    expect(read.slice(before)).toEqual([
      [
        [member.Login.channelId, 1],
        [client.Calculation.channelId, 1]
      ],
      [[client.Calculation.channelId, 2]]
    ]);
    ports.port1.close();
    ports.port2.close();
  });
});

describe('the transport of a permanently closed transport', () => {
  it('should close a channel attached after its transport disconnected', async () => {
    const driver = new MemoryDriver();
    const transport = new Transport(driver);

    new provider.CalculatorChannel(transport, NO_CHECKS);
    driver.disconnect();

    const user = new client.UserChannel(transport, NO_CHECKS);

    await expect(user.closeEvent).resolves.toBe(ErrorCode.ConnectionLoss);
  });

  it('should close what attaches to a transport it has no secret randomness for', async () => {
    const driver = new MemoryDriver();
    const transport = new Transport(driver, {
      security: new Security({
        role: SecurityRole.Initiator,
        entropy: () => {
          throw new EntropyUnavailable();
        }
      })
    });
    const user = new client.UserChannel(transport, NO_CHECKS);

    await expect(user.closeEvent).resolves.toBe(ErrorCode.Insecure);
    expect(driver.posted).toEqual([]);
  });

  it('should not become the transport of a driver when its security fails to start', () => {
    const driver = new MemoryDriver();
    const failing = new Security({
      role: SecurityRole.Initiator,
      entropy: () => {
        throw new RangeError('no bytes');
      }
    });
    const making = () => new Transport(driver, { security: failing });

    expect(making).toThrow(RangeError);

    const transport = Transport.of(driver);

    expect(transport.endReason).toBeUndefined();
    expect(transport.isAsSecureAs(undefined)).toBe(true);
  });

  it('should let hooks registered right after a channel see its first state', async () => {
    const user = new client.UserChannel(new MemoryDriver(), NO_CHECKS);
    let opened = false;

    user.onOpen(() => (opened = true));
    await settle();
    expect(opened).toBe(true);
  });
});
