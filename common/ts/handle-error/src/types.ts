export type HostCallback = (error: unknown) => boolean;
export type CtxCallback = (this: Ctx, error: unknown) => boolean;
export const ERROR_HANDLER_CALLBACK = Symbol("error_handler.callback");
export type Ctx = {
  [ERROR_HANDLER_CALLBACK]?: CtxCallback | undefined;
};
