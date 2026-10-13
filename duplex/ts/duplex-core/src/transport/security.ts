import type { SecurityLevel } from '../protocol/index.js';
import { SecureSession, type ReceiveOutcome, type Security } from '../security/secure.js';

const SECURITY_LEVEL_ORDER: readonly (SecurityLevel | undefined)[] = [undefined, 'encrypted', 'authenticated'];

export class TransportSecurity {
  static readonly NONE = new TransportSecurity(undefined, undefined, false);

  readonly #secure: SecureSession | undefined;
  readonly #level: SecurityLevel | undefined;
  readonly #signsBatches: boolean;

  private constructor(secure: SecureSession | undefined, level: SecurityLevel | undefined, signsBatches: boolean) {
    this.#secure = secure;
    this.#level = level;
    this.#signsBatches = signsBatches;
  }

  static of(security: Security | undefined): TransportSecurity {
    if (security === undefined) return TransportSecurity.NONE;

    return new TransportSecurity(new SecureSession(security), security.level, security.signsBatches);
  }

  get isSecured(): boolean {
    return this.#secure !== undefined;
  }

  get canSend(): boolean {
    return this.#secure?.isEstablished ?? true;
  }

  isAsSecureAs(level: SecurityLevel | undefined, signed: boolean): boolean {
    return (
      SECURITY_LEVEL_ORDER.indexOf(this.#level) >= SECURITY_LEVEL_ORDER.indexOf(level) &&
      (!signed || this.#signsBatches)
    );
  }

  startHandshake(): Uint8Array | undefined {
    return this.#secure?.startHandshake();
  }

  seal(batch: Uint8Array): Uint8Array | undefined {
    return this.#secure === undefined ? batch : this.#secure.seal(batch);
  }

  unseal(bytes: Uint8Array): ReceiveOutcome {
    return this.#secure!.unseal(bytes);
  }
}
