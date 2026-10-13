import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { ConnectionStateKind } from '../../../src/channel/connection/state.js';
import { FrameKind } from '../../../src/definition.generated.js';
import { CORE, ErrorCode, Role } from '../../../src/index.js';
import * as legacy from '../../legacy.server.generated.js';
import { Session } from '../../login.generated.js';
import {
  MemoryDriver,
  NO_CHECKS,
  restoreMocksAndTimers,
  settle,
  settled,
  zeroRandomValues
} from '../../support/channel.js';
import { encode } from '../../support/frames.js';
import { Directory, signingIn } from '../../support/login.js';

beforeEach(zeroRandomValues);

afterEach(restoreMocksAndTimers);

describe('the handshake', () => {
  it('should refuse a member no router accepts or denies as unhandled', async () => {
    const { client } = signingIn('ada', 't0k');
    const reason = await client.closeEvent;

    expect(reason).toBe(ErrorCode.UnhandledMessage);
    expect(client.state.kind === ConnectionStateKind.Failure ? client.state.refusal : undefined).toBeUndefined();
  });

  it('should refuse a member as its handler failed when `onOpen` throws otherwise', async () => {
    class Broken extends Directory {
      override onConnect(): Session {
        throw new Error('broken');
      }
    }

    const { client } = signingIn('ada', 't0k', new Broken());
    const reason = await client.closeEvent;

    expect(reason).toBe(ErrorCode.HandlerFailure);
  });

  it('should wait for an open settled later', async () => {
    let accept: (session: Session) => void = () => undefined;

    class Later extends Directory {
      override onConnect(): Promise<Session> {
        return new Promise((resolve) => (accept = resolve));
      }
    }

    const { client } = signingIn('ada', 't0k', new Later());

    await settled();

    const before = client.connection;

    accept(new Session({ greeting: 'later' }));
    await settled();
    expect(before).toBeUndefined();
    expect(client.state.kind === ConnectionStateKind.Active ? (client.state.ack as Session).greeting : undefined).toBe(
      'later'
    );
  });
});

describe('versions', () => {
  it('should refuse a client of an unsupported version', async () => {
    const driver = new MemoryDriver();

    new legacy.ServerChannel(driver, NO_CHECKS);
    await settle();
    driver.take();

    const protocol = { major: 1, minor: 0 };

    driver.dispatch(
      encode({
        channel: legacy.Legacy.channelId,
        from: Role.Client,
        connection: 0,
        frames: [
          {
            kind: FrameKind.Open,
            id: 9,
            core: CORE,
            protocol,
            fingerprint: legacy.Legacy.versions[0].fingerprint,
            payload: new Uint8Array(0)
          }
        ]
      })
    );
    await settle();
    expect(driver.take().flatMap((batch) => batch.frames)).toEqual([
      {
        kind: FrameKind.Refuse,
        id: 9,
        reason: ErrorCode.UnsupportedVersion,
        core: CORE,
        protocol: legacy.Legacy.version,
        majors: [1],
        payload: new Uint8Array(0)
      }
    ]);
  });
});
