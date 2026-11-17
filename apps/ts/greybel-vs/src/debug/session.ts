import {
  Breakpoint,
  InitializedEvent,
  LoggingDebugSession,
  OutputEvent,
  Scope,
  Source,
  StackFrame,
  StoppedEvent,
  TerminatedEvent,
  Thread
} from '@vscode/debugadapter';
import { DebugProtocol } from '@vscode/debugprotocol';
import { ChannelError } from 'duplex-core';
import {
  Continue,
  EditorRouter,
  Evaluate,
  EvaluationFailure,
  Exit,
  LaunchFailure,
  LogMessage,
  Pause,
  Scopes,
  SetBreakpoints,
  SetVariable,
  SourceBreakpoint,
  SourceLocation,
  StackTrace,
  Step,
  StepMode,
  StopReason,
  Terminate,
  Variables,
  type StopNotice
} from 'greybel-runtime';
import vscode, { Uri } from 'vscode';

import type { Peer } from '../launcher';
import type { Preview } from '../preview';
import type { ProjectReader, Project } from '../project';
import { RuntimeConnection } from '../runtime/connection';
import { ErrorReporter } from '../runtime/error-message';
import { Configuration } from '../settings';

const THREAD = 1;
const CONFIGURATION_TIMEOUT = 2000;

interface LaunchConfiguration extends DebugProtocol.LaunchRequestArguments {
  program: string;
  noDebug?: boolean;
  stopOnEntry?: boolean;
}

const STOP_REASONS: Record<StopReason, string> = {
  [StopReason.Entry]: 'entry',
  [StopReason.Breakpoint]: 'breakpoint',
  [StopReason.Debugger]: 'breakpoint',
  [StopReason.Step]: 'step',
  [StopReason.Pause]: 'pause'
};

export function toUri(path: string): Uri {
  return /^[a-z][\w+.-]+:/i.test(path) && !/^[a-z]:[\\/]/i.test(path) ? Uri.parse(path) : Uri.file(path);
}

export class RuntimeDebugSession extends LoggingDebugSession {
  readonly #connection: RuntimeConnection;
  readonly #projects: ProjectReader;
  readonly #preview: Preview;
  readonly #errors = new ErrorReporter();
  readonly #configurationDeferrable: Promise<void>;

  #resolveConfigurationDeferrable!: () => void;
  #ended = false;

  constructor(runtime: Peer, projects: ProjectReader, preview: Preview) {
    super();
    this.#projects = projects;
    this.#preview = preview;
    this.setDebuggerLinesStartAt1(true);
    this.setDebuggerColumnsStartAt1(true);

    this.#connection = new RuntimeConnection(runtime, new StopReporter(this), preview);
    this.#configurationDeferrable = new Promise((resolve) => {
      this.#resolveConfigurationDeferrable = resolve;
    });
  }

  protected override initializeRequest(
    response: DebugProtocol.InitializeResponse,
    _args: DebugProtocol.InitializeRequestArguments
  ): void {
    response.body = {
      ...response.body,
      supportsConfigurationDoneRequest: true,
      supportsEvaluateForHovers: false,
      supportsConditionalBreakpoints: true,
      supportsHitConditionalBreakpoints: true,
      supportsLogPoints: true,
      supportsSetVariable: true,
      supportsTerminateRequest: true,
      supportsRestartRequest: false
    };
    this.sendResponse(response);
    this.sendEvent(new InitializedEvent());
  }

  protected override configurationDoneRequest(
    response: DebugProtocol.ConfigurationDoneResponse,
    args: DebugProtocol.ConfigurationDoneArguments
  ): void {
    super.configurationDoneRequest(response, args);
    this.#resolveConfigurationDeferrable();
  }

  protected override async launchRequest(
    response: DebugProtocol.LaunchResponse,
    args: LaunchConfiguration
  ): Promise<void> {
    await Promise.race([
      this.#configurationDeferrable,
      new Promise((resolve) => setTimeout(resolve, CONFIGURATION_TIMEOUT))
    ]);

    const config = new Configuration();
    const entry = toUri(args.program);
    let project: Project;

    try {
      project = await this.#projects.read(entry);
    } catch (error) {
      this.sendErrorResponse(response, 1, error instanceof Error ? error.message : String(error));

      return;
    }

    const params = await this.#getParams(config, project);
    const environment = project.getEnvironment(
      config.get('interpreter.environmentType') === 'In-Game' ? config.get('interpreter.port') : undefined
    );

    this.#preview.clear();
    this.#preview.setLineLimits(project.getLineLimits());
    this.#connection.page.setLineLimits(project.getLineLimits());
    this.sendResponse(response);

    try {
      const exit = await this.#connection.runtime.send(
        project.createLaunch(entry, environment, params, {
          debug: !args.noDebug,
          stopOnEntry: args.stopOnEntry ?? false
        })
      );

      this.#report(exit, config);
    } catch (error) {
      this.#reportLaunchFailure(error, config);
    }

    this.#ended = true;
    this.sendEvent(new TerminatedEvent());
  }

  protected override async setBreakPointsRequest(
    response: DebugProtocol.SetBreakpointsResponse,
    args: DebugProtocol.SetBreakpointsArguments
  ): Promise<void> {
    const sourceBreakpoints = (args.breakpoints ?? []).map(
      (breakpoint) =>
        new SourceBreakpoint({
          line: this.convertClientLineToDebugger(breakpoint.line),
          condition: breakpoint.condition || undefined,
          hitCondition: breakpoint.hitCondition || undefined,
          logMessage: breakpoint.logMessage || undefined
        })
    );
    const uri = toUri(args.source.path ?? '').toString();
    const sendUnverifiedBreakpoints = (): void => {
      response.body = {
        breakpoints: sourceBreakpoints.map(
          (breakpoint) => new Breakpoint(false, this.convertDebuggerLineToClient(breakpoint.line))
        )
      };
      this.sendResponse(response);
    };

    if (this.#ended) {
      sendUnverifiedBreakpoints();

      return;
    }

    try {
      const breakpoints = await this.#connection.runtime.send(
        new SetBreakpoints({ uri, breakpoints: sourceBreakpoints })
      );

      response.body = {
        breakpoints: breakpoints.map((breakpoint) => {
          const set: DebugProtocol.Breakpoint = new Breakpoint(
            breakpoint.verified,
            this.convertDebuggerLineToClient(breakpoint.line)
          );

          if (breakpoint.message) set.message = breakpoint.message;

          return set;
        })
      };
      this.sendResponse(response);
    } catch (error) {
      if (this.#ended) {
        sendUnverifiedBreakpoints();

        return;
      }

      this.sendErrorResponse(response, { id: 3, format: String(error), showUser: false });
    }
  }

  protected override threadsRequest(response: DebugProtocol.ThreadsResponse): void {
    response.body = { threads: [new Thread(THREAD, 'main')] };
    this.sendResponse(response);
  }

  protected override async stackTraceRequest(
    response: DebugProtocol.StackTraceResponse,
    _args: DebugProtocol.StackTraceArguments
  ): Promise<void> {
    const frames = await this.#inspectOrDefault(() => this.#connection.runtime.send(new StackTrace()), []);

    response.body = {
      stackFrames: frames.map((frame) => {
        const location = frame.location;

        return location
          ? new StackFrame(
              frame.id,
              frame.name,
              this.#toSource(location.uri),
              this.convertDebuggerLineToClient(location.line),
              this.convertDebuggerColumnToClient(location.character)
            )
          : new StackFrame(frame.id, frame.name);
      }),
      totalFrames: frames.length
    };
    this.sendResponse(response);
  }

  protected override async scopesRequest(
    response: DebugProtocol.ScopesResponse,
    args: DebugProtocol.ScopesArguments
  ): Promise<void> {
    const scopes = await this.#inspectOrDefault(
      () => this.#connection.runtime.send(new Scopes({ frame: args.frameId })),
      []
    );

    response.body = {
      scopes: scopes.map((scope) => new Scope(scope.name, scope.reference, scope.name === 'Globals'))
    };
    this.sendResponse(response);
  }

  protected override async variablesRequest(
    response: DebugProtocol.VariablesResponse,
    args: DebugProtocol.VariablesArguments
  ): Promise<void> {
    const variables = await this.#inspectOrDefault(
      () => this.#connection.runtime.send(new Variables({ reference: args.variablesReference })),
      []
    );

    response.body = {
      variables: variables.map((variable) => ({
        name: variable.name,
        value: variable.value,
        type: variable.typeName,
        variablesReference: variable.reference,
        evaluateName: variable.evaluateName
      }))
    };
    this.sendResponse(response);
  }

  protected override continueRequest(
    response: DebugProtocol.ContinueResponse,
    _args: DebugProtocol.ContinueArguments
  ): void {
    this.#connection.runtime.send(new Continue());
    response.body = { allThreadsContinued: true };
    this.sendResponse(response);
  }

  protected override nextRequest(response: DebugProtocol.NextResponse, _args: DebugProtocol.NextArguments): void {
    this.#step(StepMode.Over);
    this.sendResponse(response);
  }

  protected override stepInRequest(response: DebugProtocol.StepInResponse, _args: DebugProtocol.StepInArguments): void {
    this.#step(StepMode.Into);
    this.sendResponse(response);
  }

  protected override stepOutRequest(
    response: DebugProtocol.StepOutResponse,
    _args: DebugProtocol.StepOutArguments
  ): void {
    this.#step(StepMode.Out);
    this.sendResponse(response);
  }

  protected override pauseRequest(response: DebugProtocol.PauseResponse, _args: DebugProtocol.PauseArguments): void {
    this.#connection.runtime.send(new Pause());
    this.sendResponse(response);
  }

  protected override async evaluateRequest(
    response: DebugProtocol.EvaluateResponse,
    args: DebugProtocol.EvaluateArguments
  ): Promise<void> {
    try {
      const value = await this.#connection.runtime.send(new Evaluate({ code: args.expression }));

      response.body = {
        result: value?.value ?? '',
        type: value?.typeName,
        variablesReference: value?.reference ?? 0
      };
      this.sendResponse(response);
    } catch (error) {
      this.sendErrorResponse(response, { id: 1, format: this.#getEvaluationMessage(error), showUser: false });
    }
  }

  protected override async setVariableRequest(
    response: DebugProtocol.SetVariableResponse,
    args: DebugProtocol.SetVariableArguments
  ): Promise<void> {
    try {
      const variable = await this.#connection.runtime.send(
        new SetVariable({ reference: args.variablesReference, name: args.name, value: args.value })
      );

      response.body = {
        value: variable.value,
        type: variable.typeName,
        variablesReference: variable.reference
      };
      this.sendResponse(response);
    } catch (error) {
      this.sendErrorResponse(response, { id: 2, format: this.#getEvaluationMessage(error), showUser: true });
    }
  }

  protected override terminateRequest(
    response: DebugProtocol.TerminateResponse,
    _args: DebugProtocol.TerminateArguments
  ): void {
    this.#connection.page.end();
    this.#connection.runtime.send(new Terminate());
    this.sendResponse(response);
  }

  protected override disconnectRequest(
    response: DebugProtocol.DisconnectResponse,
    _args: DebugProtocol.DisconnectArguments
  ): void {
    this.#ended = true;
    this.#connection.close();
    this.sendResponse(response);
  }

  reportLog(message: LogMessage): void {
    const event = new OutputEvent(`${message.text}\n`, 'console') as DebugProtocol.OutputEvent;

    if (message.location) {
      event.body.source = this.#toSource(message.location.uri);
      event.body.line = this.convertDebuggerLineToClient(message.location.line);
    }

    this.sendEvent(event);
  }

  reportStop(message: StopNotice): void {
    const event = new StoppedEvent(STOP_REASONS[message.reason], THREAD) as DebugProtocol.StoppedEvent;

    if (message.reason === StopReason.Debugger) event.body.description = 'Paused on debugger statement';
    this.sendEvent(event);
  }

  #step(mode: StepMode): void {
    this.#connection.runtime.send(new Step({ mode }));
  }

  async #inspectOrDefault<T>(request: () => Promise<T>, fallback: T): Promise<T> {
    try {
      return await request();
    } catch {
      return fallback;
    }
  }

  /** Returns project run parameters when defaults are active; otherwise, returns space-separated user input. */
  async #getParams(config: Configuration, project: Project): Promise<readonly string[]> {
    if (config.get('interpreter.useDefaultArgs')) return project.manifest.run.params;

    const text = await vscode.window.showInputBox({ title: 'Enter program arguments', ignoreFocusOut: true });

    return text && text.length > 0 ? text.split(' ') : [];
  }

  #report(exit: Exit, config: Configuration): void {
    if (!(exit instanceof Exit.Crash)) return;

    const stack = exit.stack.map((location) => `  at ${this.#errors.describeLocation(location)}`).join('\n');

    this.#connection.page.printError(`Runtime error: ${exit.message}${stack ? `\n${stack}` : ''}`);
    if (config.get('interpreter.silenceErrorPopups')) return;
    this.#errors.show(exit.message, exit.stack[0]);
  }

  #reportLaunchFailure(error: unknown, config: Configuration): void {
    const failure = error instanceof ChannelError ? error.failure : undefined;
    let message: string;
    let location: SourceLocation | undefined;

    if (failure instanceof LaunchFailure.Invalid) {
      message = failure.message;
      location = failure.location ?? undefined;
    } else if (failure instanceof LaunchFailure.Unavailable) {
      message = 'Grey Hack execution is not available here.';
    } else if (failure instanceof LaunchFailure.Unreachable) {
      message = failure.message;
    } else if (failure instanceof LaunchFailure.Busy) {
      message = 'A program is already active in this session.';
    } else {
      message = error instanceof Error ? error.message : String(error);
    }

    this.#connection.page.printError(`Could not start the program: ${message}`);
    if (config.get('interpreter.silenceErrorPopups')) return;
    this.#errors.show(message, location);
  }

  #toSource(uri: string): Source {
    const location = Uri.parse(uri);
    const name = location.path.split('/').pop() ?? uri;

    return new Source(name, location.scheme === 'file' ? location.fsPath : location.toString());
  }

  #getEvaluationMessage(error: unknown): string {
    const failure = error instanceof ChannelError ? error.failure : undefined;

    if (failure instanceof EvaluationFailure.CodeError) return failure.message;
    if (failure instanceof EvaluationFailure.ActiveProgram) return 'Pause the program before evaluation.';

    return String(error);
  }
}

class StopReporter extends EditorRouter {
  readonly #session: RuntimeDebugSession;

  constructor(session: RuntimeDebugSession) {
    super();
    this.#session = session;
  }

  override onStopNotice(message: StopNotice): void {
    this.#session.reportStop(message);
  }

  override onLogMessage(message: LogMessage): void {
    this.#session.reportLog(message);
  }
}
