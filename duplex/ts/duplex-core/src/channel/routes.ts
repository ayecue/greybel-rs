import { IGNORE } from '../const.js';
import { ErrorCode } from '../definition.generated.js';
import { ChannelError } from '../errors.js';
import type { AckOf, Inbound, OpenOf, PeerOf, Protocol } from '../protocol/index.js';
import type { Reply } from '../protocol/message.js';
import type { RouteContext, Routed, Router } from './router.js';

export class RouterChain<P extends Protocol, L extends PeerOf<P>> {
  #routers: readonly Router<P, L>[] = [];

  add(routers: readonly Router<P, L>[]): void {
    this.#routers = [...this.#routers, ...routers];
  }

  route<M extends Inbound<P, L>>(typeId: number, message: M, context: RouteContext): Routed<M> {
    return this.#route(this.#routers, 0, typeId, message, context);
  }

  notify(typeId: number, message: Inbound<P, L>, context: RouteContext): void {
    let routeResult: unknown;

    try {
      routeResult = this.#notify(this.#routers, 0, typeId, message, context);
    } catch {
      return;
    }

    if (typeof (routeResult as PromiseLike<unknown> | undefined)?.then !== 'function') return;
    (routeResult as PromiseLike<unknown>).then(undefined, IGNORE);
  }

  routeOpen(open: OpenOf<P>): Promise<AckOf<P>> {
    return this.#routeOpen(this.#routers, 0, open);
  }

  #route<M extends Inbound<P, L>>(
    routers: readonly Router<P, L>[],
    index: number,
    typeId: number,
    message: M,
    context: RouteContext
  ): Routed<M> {
    const router = routers[index];

    if (router === undefined) throw ChannelError.reject(ErrorCode.UnhandledMessage);

    return router.route(
      typeId,
      message,
      () => RouterChain.#toPromise(() => this.#route(routers, index + 1, typeId, message, context)),
      context
    );
  }

  #notify<M extends Inbound<P, L>>(
    routers: readonly Router<P, L>[],
    index: number,
    typeId: number,
    message: M,
    context: RouteContext
  ): Routed<M> {
    const router = routers[index];

    // Notifications have no replies, so an unrouted notification is discarded without a response.
    if (router === undefined) return undefined as Reply<M>;

    return router.route(
      typeId,
      message,
      () => RouterChain.#toPromise(() => this.#notify(routers, index + 1, typeId, message, context)),
      context
    );
  }

  #routeOpen(routers: readonly Router<P, L>[], index: number, open: OpenOf<P>): Promise<AckOf<P>> {
    const router = routers[index];

    if (router === undefined) return Promise.reject(ChannelError.reject(ErrorCode.UnhandledMessage));

    return RouterChain.#toPromise(() => router.routeOpen(open, () => this.#routeOpen(routers, index + 1, open)));
  }

  static #toPromise<T>(run: () => T | Promise<T>): Promise<T> {
    try {
      return Promise.resolve(run());
    } catch (error) {
      return Promise.reject(error);
    }
  }
}
