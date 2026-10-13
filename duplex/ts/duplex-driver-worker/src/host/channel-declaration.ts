import type { Channel, ChannelConstructor, ChannelOptions, Router } from 'duplex-core';

export type AnyChannel = Channel<any, any>;

export type RouterOf<C extends AnyChannel> = C extends Channel<infer P, infer L> ? Router<P, L> : never;

export interface ChannelDeclaration<C extends AnyChannel> {
  readonly channelClass: ChannelConstructor<C>;
  readonly routers?: readonly RouterOf<C>[];
  readonly options?: ChannelOptions;
}

export type ChannelDeclarationTable<C extends Record<string, AnyChannel>> = {
  readonly [K in keyof C]: ChannelDeclaration<C[K]>;
};
