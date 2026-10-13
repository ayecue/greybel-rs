use std::future::Future;
use std::task::{Context, Poll};

pub const DEFAULT_BATCH: usize = 1024;
pub const DEFAULT_SLICE: usize = 80_000;

pub trait Scheduler {
    fn batch_size(&self) -> usize {
        DEFAULT_BATCH
    }

    fn should_yield(&mut self, op_count: usize) -> bool;

    fn yield_now(&mut self) -> impl Future<Output = ()>;

    fn should_collect(&mut self) -> bool {
        false
    }

    fn with_yield_hook<F, Y>(self, yield_now: F) -> YieldHook<Self, F>
    where
        Self: Sized,
        F: FnMut() -> Y,
        Y: Future<Output = ()>,
    {
        YieldHook {
            inner: self,
            yield_now,
        }
    }
}

#[derive(Debug, Clone)]
pub struct YieldHook<S, F> {
    inner: S,
    yield_now: F,
}

impl<S, F, Y> Scheduler for YieldHook<S, F>
where
    S: Scheduler,
    F: FnMut() -> Y,
    Y: Future<Output = ()>,
{
    fn batch_size(&self) -> usize {
        self.inner.batch_size()
    }

    fn should_yield(&mut self, op_count: usize) -> bool {
        self.inner.should_yield(op_count)
    }

    async fn yield_now(&mut self) {
        self.inner.yield_now().await;
        (self.yield_now)().await;
    }

    fn should_collect(&mut self) -> bool {
        self.inner.should_collect()
    }
}

#[derive(Debug, Clone)]
pub struct InstructionBudget {
    per_slice: usize,
    op_count: usize,
}

impl InstructionBudget {
    #[must_use]
    pub fn new(per_slice: usize) -> Self {
        Self {
            per_slice: per_slice.max(1),
            op_count: 0,
        }
    }
}

impl Default for InstructionBudget {
    fn default() -> Self {
        Self::new(DEFAULT_SLICE)
    }
}

impl Scheduler for InstructionBudget {
    fn should_yield(&mut self, op_count: usize) -> bool {
        self.op_count += op_count;
        if self.op_count < self.per_slice {
            return false;
        }

        self.op_count = 0;

        true
    }

    fn yield_now(&mut self) -> impl Future<Output = ()> {
        let mut yielded = false;

        std::future::poll_fn(move |context: &mut Context<'_>| {
            if yielded {
                Poll::Ready(())
            } else {
                yielded = true;
                context.waker().wake_by_ref();

                Poll::Pending
            }
        })
    }
}
