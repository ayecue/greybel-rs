use crate::program::Evaluator;
use crate::protocol::{LogMessage, SourceLocation};
use crate::runtime::{Host, RuntimeState};

/// Action to take when execution reaches a breakpoint line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BreakpointAction {
    Stop,
    /// Continues without pausing for a false condition, unmet hit condition, or emitted logpoint.
    Pass,
}

impl<H: Host> RuntimeState<H> {
    /// Determines whether the breakpoint at `uri:line` pauses. False conditions, unmet hit conditions, and emitted
    /// logpoints continue; condition evaluation errors are logged and stop execution.
    pub(crate) async fn breakpoint_action(
        &self,
        uri: &str,
        line: u32,
        evaluator: &mut impl Evaluator,
    ) -> BreakpointAction {
        let Some(rule) = self.breakpoints.borrow().rule(uri, line).cloned() else {
            return BreakpointAction::Stop;
        };
        let location = SourceLocation {
            uri: uri.to_owned(),
            line,
            character: 1,
        };

        if let Some(condition) = &rule.condition {
            match evaluator.evaluate_text(&format!("not ({condition})")).await {
                Ok(Some(value)) if value.text == "1" => return BreakpointAction::Pass,
                Ok(_) => {}
                Err(failure) => {
                    self.log(
                        format!("The breakpoint's condition `{condition}` failed: {failure}"),
                        &location,
                    );

                    return BreakpointAction::Stop;
                }
            }
        }

        let hits = self.breakpoints.borrow_mut().count_hit(uri, line);

        if rule
            .hit_condition
            .is_some_and(|condition| !condition.is_met(hits))
        {
            return BreakpointAction::Pass;
        }

        if let Some(message) = &rule.log_message {
            let text = evaluator.interpolate(message).await;

            self.log(text, &location);

            return BreakpointAction::Pass;
        }

        BreakpointAction::Stop
    }

    fn log(&self, text: String, location: &SourceLocation) {
        self.host.report_log(&LogMessage {
            text: text.into(),
            location: Some(location.clone()),
        });
    }
}
