use greyhack_world_effects::Event;

pub(crate) struct First(pub(crate) u32);
impl Event for First {}

pub(crate) struct Second(pub(crate) u32);
impl Event for Second {}
