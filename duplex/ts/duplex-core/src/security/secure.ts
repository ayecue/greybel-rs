import { ed25519 } from '@noble/curves/ed25519.js';
import { concatBytes } from '@noble/hashes/utils.js';

import { Varuint } from '../binary/varint.js';
import { BATCH_CONTEXT, HANDSHAKE_HEADER_LENGTH, KEY_LENGTH, NO_SLICES, PROLOGUE, SIGNATURE_LENGTH } from '../const.js';
import { EntropyUnavailable } from '../errors.js';
import { CORE, type SecurityLevel } from '../protocol/index.js';
import { Handshake, SecurityRole, StepKind, type HandshakeOutcome, type SecurityOptions } from './handshake.js';
import { KeyPair } from './noise.js';

export enum ReceiveOutcomeKind {
  Batch,
  Reply,
  Nothing,
  Failure
}

export type ReceiveOutcome =
  | { readonly kind: ReceiveOutcomeKind.Batch; readonly batch: Uint8Array }
  | { readonly kind: ReceiveOutcomeKind.Reply; readonly reply: Uint8Array }
  | { readonly kind: ReceiveOutcomeKind.Nothing }
  | { readonly kind: ReceiveOutcomeKind.Failure };

enum SecureSessionStateKind {
  Handshake,
  Ready,
  Failure
}

type SecureSessionState =
  | { readonly kind: SecureSessionStateKind.Handshake; readonly handshake: Handshake }
  | { readonly kind: SecureSessionStateKind.Ready; readonly outcome: HandshakeOutcome }
  | { readonly kind: SecureSessionStateKind.Failure };

const NOTHING: ReceiveOutcome = { kind: ReceiveOutcomeKind.Nothing };
const FAILURE: ReceiveOutcome = { kind: ReceiveOutcomeKind.Failure };

export class Security {
  readonly options: SecurityOptions;

  constructor(options: SecurityOptions) {
    this.options = options;
  }

  static pinnedKeyVerifier(keys: readonly Uint8Array[]): (publicKey: Uint8Array) => boolean {
    return (publicKey) =>
      keys.some((key) => key.length === publicKey.length && key.every((byte, index) => byte === publicKey[index]));
  }

  static publicKey(secret: Uint8Array): Uint8Array {
    return ed25519.getPublicKey(secret);
  }

  get level(): SecurityLevel {
    return this.options.verify === undefined ? 'encrypted' : 'authenticated';
  }

  get signsBatches(): boolean {
    return this.options.signBatches === true;
  }
}

export class SecureSession {
  readonly #options: SecurityOptions;
  #state: SecureSessionState;

  constructor(security: Security) {
    const { options } = security;

    this.#options = options;

    const staticKey = new KeyPair(SecureSession.#randomKey(options));
    const ephemeralKey = new KeyPair(SecureSession.#randomKey(options));
    const handshake = new Handshake(options, concatBytes(PROLOGUE, Uint8Array.of(CORE.major)), staticKey, ephemeralKey);

    this.#state = { kind: SecureSessionStateKind.Handshake, handshake };
  }

  static #randomKey({ entropy }: SecurityOptions): Uint8Array {
    const key = new Uint8Array(KEY_LENGTH);

    if (entropy !== undefined) {
      entropy(key);

      return key;
    }

    const crypto = globalThis.crypto as Crypto | undefined;

    if (crypto?.getRandomValues === undefined) throw new EntropyUnavailable();

    return crypto.getRandomValues(key);
  }

  startHandshake(): Uint8Array | undefined {
    if (this.#options.role !== SecurityRole.Initiator || this.#state.kind !== SecureSessionStateKind.Handshake)
      return undefined;

    return SecureSession.#handshakeBatch(this.#state.handshake.start());
  }

  get isEstablished(): boolean {
    return this.#state.kind === SecureSessionStateKind.Ready;
  }

  seal(batch: Uint8Array): Uint8Array | undefined {
    if (this.#state.kind !== SecureSessionStateKind.Ready) return undefined;

    const { outcome } = this.#state;
    const headerLength = SecureSession.#headerLength(batch);

    if (headerLength === undefined) return undefined;

    const header = batch.subarray(0, headerLength);
    const sealedBatch = concatBytes(header, outcome.sendCipher.encrypt(header, batch.subarray(headerLength)));
    const { identity } = this.#options;

    if (!outcome.signsBatches) return sealedBatch;
    if (identity === undefined) return undefined;

    return concatBytes(sealedBatch, ed25519.sign(concatBytes(BATCH_CONTEXT, sealedBatch), identity));
  }

  unseal(sealedBatch: Uint8Array): ReceiveOutcome {
    try {
      return this.#unsealOrThrow(sealedBatch);
    } catch {
      return this.#enterFailure();
    }
  }

  #unsealOrThrow(sealedBatch: Uint8Array): ReceiveOutcome {
    const headerLength = SecureSession.#headerLength(sealedBatch);

    if (headerLength === undefined) return NOTHING;
    if (headerLength === HANDSHAKE_HEADER_LENGTH && sealedBatch[1] === NO_SLICES)
      return this.#readHandshake(sealedBatch.subarray(headerLength));
    if (this.#state.kind !== SecureSessionStateKind.Ready) return NOTHING;

    const { outcome } = this.#state;
    let end = sealedBatch.length;

    if (outcome.signsBatches) {
      end -= SIGNATURE_LENGTH;

      const remote = outcome.remoteIdentity;

      if (remote === undefined || end < headerLength) return this.#enterFailure();

      const signatureInput = concatBytes(BATCH_CONTEXT, sealedBatch.subarray(0, end));

      if (!ed25519.verify(sealedBatch.subarray(end), signatureInput, remote)) return this.#enterFailure();
    }

    const header = sealedBatch.subarray(0, headerLength);
    const body = outcome.receiveCipher.decrypt(header, sealedBatch.subarray(headerLength, end));

    return body === undefined
      ? this.#enterFailure()
      : { kind: ReceiveOutcomeKind.Batch, batch: concatBytes(header, body) };
  }

  #readHandshake(message: Uint8Array): ReceiveOutcome {
    if (this.#state.kind !== SecureSessionStateKind.Handshake) return NOTHING;

    const step = this.#state.handshake.readMessage(message);

    if (step.kind === StepKind.Complete) this.#state = { kind: SecureSessionStateKind.Ready, outcome: step.outcome };

    return step.message === undefined
      ? NOTHING
      : { kind: ReceiveOutcomeKind.Reply, reply: SecureSession.#handshakeBatch(step.message) };
  }

  #enterFailure(): ReceiveOutcome {
    this.#state = { kind: SecureSessionStateKind.Failure };

    return FAILURE;
  }

  static #handshakeBatch(message: Uint8Array): Uint8Array {
    return concatBytes(Uint8Array.of(CORE.major, NO_SLICES), message);
  }

  static #headerLength(batch: Uint8Array): number | undefined {
    return batch.length < 1 ? undefined : Varuint.getEnd(batch, 1);
  }
}
