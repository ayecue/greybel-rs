import { readFileSync } from 'node:fs';
import { join } from 'node:path';

import { inject } from 'vitest';

import { Transport, Security, type SecurityOptions } from '../../src/index.js';
import { SecurityRole } from '../../src/security/handshake.js';
import { MemoryLink } from './channel.js';

export const CLIENT_IDENTITY = new Uint8Array(32).fill(1);

export const PROVIDER_IDENTITY = new Uint8Array(32).fill(2);

export const SECURE_FIXTURES = [
  'secureFirst',
  'secureSecond',
  'secureThird',
  'secureClientBatch',
  'secureProviderBatch'
];

export const fixtures: Record<string, string> = JSON.parse(
  readFileSync(join(inject('duplex'), 'conformance/fixtures.json'), 'utf8')
);

export function zeros(bytes: Uint8Array): void {
  bytes.fill(0);
}

export function identified(
  role: SecurityOptions['role'],
  identity: Uint8Array,
  trusted: Uint8Array,
  signBatches = false
): Security {
  const entropy = signBatches ? zeros : undefined;

  return new Security({
    role,
    identity,
    verify: Security.pinnedKeyVerifier([Security.publicKey(trusted)]),
    signBatches,
    entropy
  });
}

export function transports(clientSecurity: Security, providerSecurity: Security) {
  const link = new MemoryLink();
  const near = new Transport(link.client, { security: clientSecurity });
  const far = new Transport(link.provider, { security: providerSecurity });

  return { link, near, far };
}

export function authenticated(signBatches = false) {
  return transports(
    identified(SecurityRole.Initiator, CLIENT_IDENTITY, PROVIDER_IDENTITY, signBatches),
    identified(SecurityRole.Responder, PROVIDER_IDENTITY, CLIENT_IDENTITY, signBatches)
  );
}
