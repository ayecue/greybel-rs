use std::cell::{Cell, RefCell};
use std::rc::Rc;

use greybel_documents::{PathResolution, Uri};
use greybel_interpreter::DebugControl;

use super::host::Host;
use super::yielder::Yielder;
use crate::breakpoints::BreakpointTable;
use crate::protocol::{Launch, StopReason};
use crate::uri::EditorUri as _;

/// State shared between controllers and the active program.
pub(crate) struct RuntimeState<H: Host> {
    pub host: H,
    pub control: Rc<DebugControl>,
    /// Breakpoint rules shared with controllers and the active program.
    pub breakpoints: RefCell<BreakpointTable>,
    /// Pending controller-requested step or pause stop, otherwise `None`.
    pub stop_reason: Cell<Option<StopReason>>,
    pub yielder: RefCell<Option<Yielder>>,
}

impl<H: Host> RuntimeState<H> {
    pub(super) fn new(host: H) -> Self {
        Self {
            host,
            control: DebugControl::new(),
            breakpoints: RefCell::default(),
            stop_reason: Cell::new(None),
            yielder: RefCell::new(None),
        }
    }

    /// Builds the launch document provider. Absolute imports resolve from the launch root, or the entry folder when
    /// no root exists.
    pub(crate) fn provider(&self, launch: &Launch<'_>) -> H::Provider {
        let parse = |uri: &str| Uri::parse(&uri.canonical()).ok();
        let resolution = launch.resolution();
        let resolution = match parse(&launch.entry) {
            Some(entry) => {
                resolution.with_base_folder(launch.root.as_deref().and_then(parse), &entry)
            }
            None => resolution,
        };

        self.host.provider(resolution)
    }
}

impl Launch<'_> {
    /// Builds import resolution using the launch's extensions.
    pub(crate) fn resolution(&self) -> PathResolution {
        PathResolution::new(self.extensions.iter().map(ToString::to_string))
    }
}
