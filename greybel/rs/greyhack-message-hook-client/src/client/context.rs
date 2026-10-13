use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use duplex_core::Bytes;
use greybel_source_map::SourceMap;
use serde_json::Value;

use super::{Client, ClientError, Subscription};
use crate::protocol::greyhack::{
    ContextDebugModeRpc, ContextEvaluateRpc, ContextInjectCodeRpc, ContextPassBreakpointRpc,
    ContextRuntimeStateServerRpc, ContextSetBreakpointsRpc, ContextStepRpc, CreateContextRpc,
    DisposeContextRpc, InputUserServerRpc,
};
use crate::protocol::{
    ContextBreakpoint, DebugValue, GreybelNotification, Payload, StackItem, StepMode,
};

/// Placeholder terminal PID before `CreatedContext`; actual game PIDs are positive.
const UNKNOWN_PID: i32 = i32::MIN;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextOptions {
    /// Program binary source written by the plugin to `<home>/<program_name>` for this run.
    pub source: String,
    /// Original files for [`Self::source`] lines, used by runtime errors, stacks, and breakpoints.
    pub source_map: SourceMap,
    /// Marks [`Self::source`] as strict GreyScript compiled by Grey Hack, where `debugger` is an identifier rather
    /// than a breakpoint.
    pub strict_mode: bool,
    /// Values for the program's `params`.
    pub params: Vec<String>,
    pub filepath: String,
    pub basepath: String,
    pub program_name: String,
    pub debug_mode: bool,
    pub breakpoints: Vec<ContextBreakpoint>,
}

impl ContextOptions {
    #[must_use]
    pub fn new(
        source: impl Into<String>,
        filepath: impl Into<String>,
        basepath: impl Into<String>,
    ) -> Self {
        Self {
            source: source.into(),
            source_map: SourceMap::default(),
            strict_mode: false,
            params: Vec::new(),
            filepath: filepath.into(),
            basepath: basepath.into(),
            program_name: "myprogram".to_owned(),
            debug_mode: true,
            breakpoints: Vec::new(),
        }
    }
}

/// Game debugger evaluation result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Evaluation {
    /// Value produced by an expression.
    Value(DebugValue),
    /// Statements finished without producing a value.
    Statements,
    /// Error message from compilation or execution.
    Failure(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RuntimeState {
    pub variables: Value,
    pub stacktrace: Vec<StackItem>,
}

#[derive(Clone)]
pub struct Context {
    client: Client,
    context_id: String,
    terminal_pid: i32,
    /// Buffers context-creation events until the first [`Context::events`] call consumes the subscription.
    initial_events: Arc<Mutex<Option<Subscription>>>,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Context")
            .field("context_id", &self.context_id)
            .field("terminal_pid", &self.terminal_pid)
            .finish_non_exhaustive()
    }
}

impl Context {
    pub(super) async fn create(
        client: Client,
        options: ContextOptions,
    ) -> Result<Self, ClientError> {
        let context_id = client.new_id();
        let request = CreateContextRpc {
            context_id: context_id.clone().into(),
            filepath: options.filepath.into(),
            basepath: options.basepath.into(),
            program_name: options.program_name.into(),
            source: Bytes::from_text(&options.source),
            params: Bytes::from_json(&options.params),
            source_map: Bytes::from_json(&options.source_map),
            strict_mode: options.strict_mode,
            debug_mode: options.debug_mode,
            breakpoints: Bytes::from_json(&options.breakpoints),
        };

        let initial_events = {
            let id = context_id.clone();
            let pid = AtomicI32::new(UNKNOWN_PID);

            client.subscribe(move |n| {
                if let GreybelNotification::CreatedContextRpc(creation) = n
                    && creation.context_id == id
                {
                    pid.store(creation.terminal_pid, Ordering::Relaxed);

                    return false;
                }

                let pid = pid.load(Ordering::Relaxed);

                n.context_id() == Some(id.as_str()) || (pid != UNKNOWN_PID && n.matches_pid(pid))
            })
        };
        let target_id = context_id.clone();
        let notification = client
            .request(
                &request,
                move |n| matches!(n, GreybelNotification::CreatedContextRpc(m) if m.context_id == target_id),
                None,
            )
            .await?;
        let GreybelNotification::CreatedContextRpc(reply) = notification else {
            return Err(ClientError::UnexpectedResponse(notification.name()));
        };

        Ok(Self {
            client,
            context_id: reply.context_id.into_owned(),
            terminal_pid: reply.terminal_pid,
            initial_events: Arc::new(Mutex::new(Some(initial_events))),
        })
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.context_id
    }

    #[must_use]
    pub fn terminal_pid(&self) -> i32 {
        self.terminal_pid
    }

    /// Subscribes to context events. The first subscription includes creation-time buffered events; subsequent
    /// subscriptions begin at their call time.
    #[must_use]
    pub fn events(&self) -> Subscription {
        let initial = self
            .initial_events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();

        if let Some(initial) = initial {
            return initial;
        }

        let (id, pid) = (self.context_id.clone(), self.terminal_pid);

        self.client
            .subscribe(move |n| n.belongs_to_context(&id, pid))
    }

    pub async fn set_debug_mode(&self, active: bool) -> Result<(), ClientError> {
        let request = ContextDebugModeRpc {
            context_id: self.context_id.clone().into(),
            active_breakpoint: active,
        };

        self.client.send(&request).await
    }

    pub async fn inject_code(&self, source: &str) -> Result<(), ClientError> {
        let request = ContextInjectCodeRpc {
            context_id: self.context_id.clone().into(),
            source: Bytes::from_text(source),
        };

        self.client.send(&request).await
    }

    /// Requests a debugger step with `mode`.
    pub async fn step(&self, mode: StepMode) -> Result<(), ClientError> {
        self.client
            .send(&ContextStepRpc {
                context_id: self.context_id.clone().into(),
                mode: mode as i32,
            })
            .await
    }

    /// Resumes past the current breakpoint while retaining any pending step.
    pub async fn pass_breakpoint(&self) -> Result<(), ClientError> {
        self.client
            .send(&ContextPassBreakpointRpc {
                context_id: self.context_id.clone().into(),
            })
            .await
    }

    /// Replaces the running context's breakpoints.
    pub async fn set_breakpoints(
        &self,
        breakpoints: &[ContextBreakpoint],
    ) -> Result<(), ClientError> {
        self.client
            .send(&ContextSetBreakpointsRpc {
                context_id: self.context_id.clone().into(),
                breakpoints: Bytes::from_json(breakpoints),
            })
            .await
    }

    /// Evaluates `code` in the stopped context and returns an expression value, `Statements`, or `Failure`.
    pub async fn evaluate(&self, code: &str) -> Result<Evaluation, ClientError> {
        let id = self.context_id.clone();
        let notification = self
            .client
            .request(
                &ContextEvaluateRpc {
                    context_id: id.clone().into(),
                    code: Bytes::from_text(code),
                },
                move |n| matches!(n, GreybelNotification::ContextEvaluatedRpc(m) if m.context_id == id),
                None,
            )
            .await?;
        let GreybelNotification::ContextEvaluatedRpc(reply) = notification else {
            return Err(ClientError::UnexpectedResponse(notification.name()));
        };

        Ok(if reply.failed {
            Evaluation::Failure(reply.message.into_owned())
        } else if reply.has_value {
            Evaluation::Value(reply.value.json().unwrap_or_default())
        } else {
            Evaluation::Statements
        })
    }

    pub async fn send_input(&self, content: &str) -> Result<(), ClientError> {
        let request = InputUserServerRpc {
            zip_linea: Bytes::from_text(content),
            terminal_pid: self.terminal_pid,
        };

        self.client.send(&request).await
    }

    pub async fn runtime_state(&self) -> Result<RuntimeState, ClientError> {
        let id = self.context_id.clone();
        let request = ContextRuntimeStateServerRpc {
            context_id: id.clone().into(),
        };
        let notification = self
            .client
            .request(
                &request,
                move |n| matches!(n, GreybelNotification::ContextRuntimeStateClientRpc(m) if m.context_id == id),
                None,
            )
            .await?;

        match notification {
            GreybelNotification::ContextRuntimeStateClientRpc(m) => Ok(RuntimeState {
                variables: m.states.json().unwrap_or_default(),
                stacktrace: m.stacktrace.json().unwrap_or_default(),
            }),
            other => Err(ClientError::UnexpectedResponse(other.name())),
        }
    }

    pub async fn variables(&self) -> Result<Value, ClientError> {
        self.runtime_state().await.map(|state| state.variables)
    }

    pub async fn dispose(&self) -> Result<(), ClientError> {
        self.client
            .send(&DisposeContextRpc {
                context_id: self.context_id.clone().into(),
            })
            .await
    }
}
