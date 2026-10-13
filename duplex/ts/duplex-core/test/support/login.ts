import { onTestFinished } from 'vitest';

import { ChannelError } from '../../src/index.js';
import { Credentials, Denial, Session } from '../login.generated.js';
import * as member from '../login.member.generated.js';
import * as service from '../login.service.generated.js';
import { NO_CHECKS, PortDriver } from './channel.js';

export class Directory extends service.ServiceRouter {
  override onConnect({ user, token }: Credentials): Session | Promise<Session> {
    if (user !== 'ada') throw ChannelError.deny(Denial.UnknownUser);
    if (token !== 't0k') throw ChannelError.deny(Denial.WrongToken);

    return new Session({ greeting: `hello, ${user}` });
  }

  override onWhoami(): string {
    return 'ada';
  }
}

export class Doorman extends service.ServiceRouter {
  override onConnect({ user }: Credentials): Session {
    return new Session({ greeting: `hello, ${user}` });
  }

  override onWhoami(): string {
    return 'ada';
  }
}

export function signingIn(user: string, token: string, ...routers: service.ServiceRouter[]) {
  const ports = new MessageChannel();
  const provider = new service.ServiceChannel(new PortDriver(ports.port2), NO_CHECKS).use(...routers);
  const client = new member.MemberChannel(new PortDriver(ports.port1), new Credentials({ user, token }), NO_CHECKS);

  onTestFinished(() => {
    client.close();
    provider.close();
    ports.port1.close();
    ports.port2.close();
  });

  return { client, provider };
}
