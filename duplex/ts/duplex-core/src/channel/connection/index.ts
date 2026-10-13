import type { ErrorCode } from '../../definition.generated.js';
import type { Control } from '../../frame/control.js';
import type { Migration } from '../../migration/index.js';
import type { Session } from '../../migration/session.js';
import type { IHandshake } from '../../protocol/index.js';
import type { OutgoingFlow } from '../flows/outbound.js';
import type { HealthCheck } from '../options.js';
import { ConnectionContext, type QueueItem } from './context.js';
import { Health } from './health.js';
import type { VersionTable } from './version-table.js';
import { ConnectionStateKind, type ConnectionListener, type ConnectionState, type OpenOutcome } from './state.js';

export class Connection {
  #state: ConnectionState;
  readonly #context: ConnectionContext;
  #reportedHealth = Health.Healthy;

  constructor(
    initialState: ConnectionState,
    flavour: VersionTable,
    handshake: IHandshake | undefined,
    healthCheck: HealthCheck | undefined,
    openPayload: Uint8Array | undefined,
    listener: ConnectionListener
  ) {
    this.#state = initialState;
    this.#context = new ConnectionContext(
      flavour,
      handshake,
      healthCheck,
      openPayload,
      listener,
      (state) => (this.#state = state)
    );
  }

  get state(): ConnectionState {
    return this.#state;
  }

  get activeId(): number | undefined {
    return this.#state.kind === ConnectionStateKind.Active ? this.#state.id : undefined;
  }

  get session(): Session | undefined {
    return this.#state.kind === ConnectionStateKind.Active ? this.#state.session : undefined;
  }

  get reason(): ErrorCode | undefined {
    return this.#state.reason;
  }

  get health(): Health {
    const healthCheck = this.#context.healthCheck;

    if (healthCheck === undefined || this.#state.kind !== ConnectionStateKind.Active) return Health.Healthy;

    return this.#context.missedChecks > healthCheck.unhealthyThreshold ? Health.Unhealthy : Health.Healthy;
  }

  get queuedCount(): number {
    return this.#context.queue.length;
  }

  start(): void {
    const state = this.#state;

    if (state.kind === ConnectionStateKind.Init) state.start(this.#context);
    this.#context.listener.onEnter(state);
  }

  onControl(control: Control, connection: number | undefined): void {
    this.#state.onControl(control, connection, this.#context);
    this.#reportHealth();
  }

  runHealthCheck(): void {
    this.#state.onHealthCheck(this.#context);
    this.#reportHealth();
  }

  onOpenResult(id: number, outcome: OpenOutcome, random: Uint8Array): void {
    this.#state.onOpenResult(id, outcome, random, this.#context);
    this.#reportHealth();
  }

  close(reason: ErrorCode, notifyRemote = true): void {
    this.#state.close(reason, notifyRemote, this.#context);
    this.#reportHealth();
  }

  onActivity(): void {
    if (this.#context.missedChecks === 0) {
      this.#context.activitySinceCheck = true;

      return;
    }

    this.#context.recordActivity();
    this.#reportHealth();
  }

  addMigration(migration: Migration): void {
    if (this.#context.isClient) return;
    this.#context.migrations.add(migration);
  }

  getActiveRoute(): { connection: number; session: Session } | undefined {
    return this.#state.getActiveRoute();
  }

  queue(item: QueueItem): void {
    this.#context.queue.push(item);
  }

  withdraw(flow: OutgoingFlow): boolean {
    const queue = this.#context.queue;
    const index = queue.findIndex((entry) => entry.flow === flow);

    if (index < 0) return false;
    queue.splice(index, 1);

    return true;
  }

  #reportHealth(): void {
    const health = this.health;

    if (health === this.#reportedHealth) return;
    this.#reportedHealth = health;
    if (this.activeId === undefined) return;
    this.#context.listener.onHealth(health);
  }
}
