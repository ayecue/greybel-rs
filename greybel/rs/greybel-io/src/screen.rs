use crate::types::Output;

pub trait Screen {
    fn show(&self, batch: &[Output]);
}
