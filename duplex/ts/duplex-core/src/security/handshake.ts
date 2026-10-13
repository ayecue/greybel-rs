import { ed25519 } from '@noble/curves/ed25519.js';
import { concatBytes } from '@noble/hashes/utils.js';

import {
  ENCRYPTED_KEY_LENGTH,
  HANDSHAKE_FLAGS,
  IDENTITY_FLAG,
  IDENTITY_CONTEXT,
  KEY_LENGTH,
  NO_BYTES,
  SIGNATURE_LENGTH,
  SIGNS_BATCHES
} from '../const.js';
import { CipherState, KeyPair, SymmetricState } from './noise.js';

export enum SecurityRole {
  Initiator,
  Responder
}

export interface SecurityOptions {
  readonly role: SecurityRole;
  readonly identity?: Uint8Array;
  readonly verify?: (publicKey: Uint8Array) => boolean;
  readonly signBatches?: boolean;
  readonly entropy?: (bytes: Uint8Array<ArrayBuffer>) => void;
}

export interface HandshakeOutcome {
  readonly sendCipher: CipherState;
  readonly receiveCipher: CipherState;
  readonly remoteIdentity: Uint8Array | undefined;
  readonly signsBatches: boolean;
}

export enum StepKind {
  Send,
  Complete
}

export type Step =
  | { readonly kind: StepKind.Send; readonly message: Uint8Array }
  | { readonly kind: StepKind.Complete; readonly message: Uint8Array | undefined; readonly outcome: HandshakeOutcome };

export class Refusal extends Error {}

export class Handshake {
  readonly #state: SymmetricState;
  readonly #options: SecurityOptions;
  readonly #staticKey: KeyPair;
  readonly #ephemeralKey: KeyPair;
  #turn = 0;

  constructor(options: SecurityOptions, prologue: Uint8Array, staticKey: KeyPair, ephemeralKey: KeyPair) {
    this.#state = new SymmetricState(prologue);
    this.#options = options;
    this.#staticKey = staticKey;
    this.#ephemeralKey = ephemeralKey;
  }

  start(): Uint8Array {
    const ephemeral = this.#ephemeralKey.publicKey;

    this.#state.mixHash(ephemeral);
    this.#turn = 1;

    return concatBytes(ephemeral, this.#state.encryptAndHash(NO_BYTES));
  }

  readMessage(message: Uint8Array): Step {
    const { role } = this.#options;

    if (role === SecurityRole.Responder && this.#turn === 0) return this.#readInitiatorEphemeral(message);
    if (role === SecurityRole.Initiator && this.#turn === 1) return this.#readResponderKeys(message);
    if (role === SecurityRole.Responder && this.#turn === 2) return this.#readInitiatorStatic(message);
    throw new Refusal('a message out of turn');
  }

  #readInitiatorEphemeral(message: Uint8Array): Step {
    const state = this.#state;
    const remoteEphemeral = Handshake.#readKey(message);

    state.mixHash(remoteEphemeral);
    Handshake.#requirePlaintext(state.decryptAndHash(message.subarray(KEY_LENGTH)));

    const ephemeral = this.#ephemeralKey.publicKey;

    state.mixHash(ephemeral);
    state.mixKey(this.#ephemeralKey.getSharedSecret(remoteEphemeral));

    const encryptedStatic = state.encryptAndHash(this.#staticKey.publicKey);

    state.mixKey(this.#staticKey.getSharedSecret(remoteEphemeral));

    const identityPayload = state.encryptAndHash(this.#buildIdentityPayload());

    this.#turn = 2;

    return { kind: StepKind.Send, message: concatBytes(ephemeral, encryptedStatic, identityPayload) };
  }

  #readResponderKeys(message: Uint8Array): Step {
    const state = this.#state;
    const remoteEphemeral = Handshake.#readKey(message);

    state.mixHash(remoteEphemeral);
    state.mixKey(this.#ephemeralKey.getSharedSecret(remoteEphemeral));

    const remainder = message.subarray(KEY_LENGTH);
    const remoteStatic = this.#readStatic(remainder);

    state.mixKey(this.#ephemeralKey.getSharedSecret(remoteStatic));

    const remote = this.#validateIdentityPayload(remainder.subarray(ENCRYPTED_KEY_LENGTH));
    const encryptedStatic = state.encryptAndHash(this.#staticKey.publicKey);

    state.mixKey(this.#staticKey.getSharedSecret(remoteEphemeral));

    const identityPayload = state.encryptAndHash(this.#buildIdentityPayload());
    const [sendCipher, receiveCipher] = state.split();

    return {
      kind: StepKind.Complete,
      message: concatBytes(encryptedStatic, identityPayload),
      outcome: this.#handshakeOutcome(sendCipher, receiveCipher, remote)
    };
  }

  #readInitiatorStatic(message: Uint8Array): Step {
    const remoteStatic = this.#readStatic(message);

    this.#state.mixKey(this.#ephemeralKey.getSharedSecret(remoteStatic));

    const remote = this.#validateIdentityPayload(message.subarray(ENCRYPTED_KEY_LENGTH));
    const [receiveCipher, sendCipher] = this.#state.split();

    return {
      kind: StepKind.Complete,
      message: undefined,
      outcome: this.#handshakeOutcome(sendCipher, receiveCipher, remote)
    };
  }

  #handshakeOutcome(
    sendCipher: CipherState,
    receiveCipher: CipherState,
    remote: Uint8Array | undefined
  ): HandshakeOutcome {
    return { sendCipher, receiveCipher, remoteIdentity: remote, signsBatches: this.#options.signBatches === true };
  }

  #readStatic(message: Uint8Array): Uint8Array {
    if (message.length < ENCRYPTED_KEY_LENGTH) throw new Refusal('a message cut short');

    return Handshake.#requirePlaintext(this.#state.decryptAndHash(message.subarray(0, ENCRYPTED_KEY_LENGTH)));
  }

  #validateIdentityPayload(ciphertext: Uint8Array): Uint8Array | undefined {
    const signatureInput = Handshake.#signatureInput(this.#state.hash);
    const identityPayload = Handshake.#requirePlaintext(this.#state.decryptAndHash(ciphertext));
    const flags = identityPayload[0];
    const signsBatches = this.#options.signBatches === true;

    if (flags === undefined || (flags & ~HANDSHAKE_FLAGS) !== 0 || ((flags & SIGNS_BATCHES) !== 0) !== signsBatches) {
      throw new Refusal('an identity payload with other flags');
    }

    let remote: Uint8Array | undefined;

    if ((flags & IDENTITY_FLAG) !== 0) {
      remote = identityPayload.slice(1, 1 + KEY_LENGTH);

      const signature = identityPayload.subarray(1 + KEY_LENGTH);

      if (
        remote.length !== KEY_LENGTH ||
        signature.length !== SIGNATURE_LENGTH ||
        !ed25519.verify(signature, signatureInput, remote)
      ) {
        throw new Refusal('a forged identity signature');
      }
    }

    const { verify } = this.#options;
    const trusted = verify === undefined || (remote !== undefined && verify(remote));

    if (!trusted || (signsBatches && remote === undefined)) throw new Refusal('an untrusted identity');

    return remote;
  }

  #buildIdentityPayload(): Uint8Array {
    const flags = this.#options.signBatches === true ? SIGNS_BATCHES : 0;
    const { identity } = this.#options;

    if (identity === undefined) return Uint8Array.of(flags);

    const signature = ed25519.sign(Handshake.#signatureInput(this.#state.hash), identity);

    return concatBytes(Uint8Array.of(flags | IDENTITY_FLAG), ed25519.getPublicKey(identity), signature);
  }

  static #signatureInput(hash: Uint8Array): Uint8Array {
    return concatBytes(IDENTITY_CONTEXT, hash);
  }

  static #readKey(message: Uint8Array): Uint8Array {
    if (message.length < KEY_LENGTH) throw new Refusal('a message cut short');

    return message.slice(0, KEY_LENGTH);
  }

  static #requirePlaintext<T>(value: T | undefined): T {
    if (value === undefined) throw new Refusal('a forged message');

    return value;
  }
}
