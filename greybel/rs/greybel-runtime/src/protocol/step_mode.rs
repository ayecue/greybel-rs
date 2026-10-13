use super::StepMode;

impl From<StepMode> for greybel_interpreter::StepMode {
    fn from(mode: StepMode) -> Self {
        match mode {
            StepMode::Into => Self::Into,
            StepMode::Over => Self::Over,
            StepMode::Out => Self::Out,
        }
    }
}

#[cfg(feature = "game")]
impl From<StepMode> for greyhack_message_hook_client::StepMode {
    fn from(mode: StepMode) -> Self {
        match mode {
            StepMode::Into => Self::Into,
            StepMode::Over => Self::Over,
            StepMode::Out => Self::Out,
        }
    }
}
