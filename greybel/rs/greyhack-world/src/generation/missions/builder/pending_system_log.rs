use crate::generation::missions::config::SystemLogSpec;

pub(super) struct PendingSystemLog {
    pub(super) beat: i32,
    pub(super) to_computer: i32,
    pub(super) spec: SystemLogSpec,
}
