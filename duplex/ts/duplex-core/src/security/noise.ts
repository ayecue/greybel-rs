import { chacha20poly1305 } from '@noble/ciphers/chacha.js';
import { x25519 } from '@noble/curves/ed25519.js';
import { hmac } from '@noble/hashes/hmac.js';
import { sha256 } from '@noble/hashes/sha2.js';
import { concatBytes } from '@noble/hashes/utils.js';

import { COUNTER_OFFSET, NO_BYTES, NONCE_LENGTH, PROTOCOL_NAME } from '../const.js';

export class CipherState {
  readonly #key: Uint8Array | undefined;
  #counter = 0n;

  constructor(key?: Uint8Array) {
    this.#key = key;
  }

  encrypt(associatedData: Uint8Array, plaintext: Uint8Array): Uint8Array {
    if (this.#key === undefined) return plaintext.slice();

    const ciphertext = chacha20poly1305(this.#key, CipherState.#nonceOf(this.#counter), associatedData).encrypt(
      plaintext
    );

    this.#counter++;

    return ciphertext;
  }

  decrypt(associatedData: Uint8Array, ciphertext: Uint8Array): Uint8Array | undefined {
    if (this.#key === undefined) return ciphertext.slice();
    try {
      const plaintext = chacha20poly1305(this.#key, CipherState.#nonceOf(this.#counter), associatedData).decrypt(
        ciphertext
      );

      this.#counter++;

      return plaintext;
    } catch {
      return undefined;
    }
  }

  static #nonceOf(counter: bigint): Uint8Array {
    const nonce = new Uint8Array(NONCE_LENGTH);

    new DataView(nonce.buffer).setBigUint64(COUNTER_OFFSET, counter, true);

    return nonce;
  }
}

export class SymmetricState {
  #chainingKey: Uint8Array = PROTOCOL_NAME.slice();
  #hash: Uint8Array = PROTOCOL_NAME.slice();
  #cipher = new CipherState();

  constructor(prologue: Uint8Array) {
    this.mixHash(prologue);
  }

  get hash(): Uint8Array {
    return this.#hash;
  }

  mixHash(data: Uint8Array): void {
    this.#hash = sha256(concatBytes(this.#hash, data));
  }

  mixKey(input: Uint8Array): void {
    const [chainingKey, key] = SymmetricState.#hkdf(this.#chainingKey, input);

    this.#chainingKey = chainingKey;
    this.#cipher = new CipherState(key);
  }

  encryptAndHash(plaintext: Uint8Array): Uint8Array {
    const ciphertext = this.#cipher.encrypt(this.#hash, plaintext);

    this.mixHash(ciphertext);

    return ciphertext;
  }

  decryptAndHash(ciphertext: Uint8Array): Uint8Array | undefined {
    const plaintext = this.#cipher.decrypt(this.#hash, ciphertext);

    if (plaintext !== undefined) this.mixHash(ciphertext);

    return plaintext;
  }

  split(): [CipherState, CipherState] {
    const [initiator, responder] = SymmetricState.#hkdf(this.#chainingKey, NO_BYTES);

    return [new CipherState(initiator), new CipherState(responder)];
  }

  static #hkdf(chainingKey: Uint8Array, input: Uint8Array): [Uint8Array, Uint8Array] {
    const temporary = hmac(sha256, chainingKey, input);
    const first = hmac(sha256, temporary, Uint8Array.of(1));
    const second = hmac(sha256, temporary, concatBytes(first, Uint8Array.of(2)));

    return [first, second];
  }
}

export class KeyPair {
  readonly publicKey: Uint8Array;
  readonly #secret: Uint8Array;

  constructor(secret: Uint8Array) {
    this.#secret = secret;
    this.publicKey = x25519.getPublicKey(secret);
  }

  getSharedSecret(publicKey: Uint8Array): Uint8Array {
    return x25519.getSharedSecret(this.#secret, publicKey);
  }
}
