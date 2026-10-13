const STREAM_BUDGET: u32 = 1024 * 1024;
pub(crate) const INITIAL_CREDIT: u32 = 1;
const ASSUMED_ITEM_SIZE: u32 = 4 * 1024;

pub(crate) struct ReceiveWindow {
    limit: u32,
    item_size: Option<u32>,
    largest: u32,
    window: u32,
    outstanding_credit: u32,
}

impl ReceiveWindow {
    pub(crate) fn new(limit: u32, item_size: Option<u32>) -> Self {
        let limit = limit.max(1);

        Self {
            limit,
            item_size,
            largest: 0,
            window: Self::fit(item_size.unwrap_or(ASSUMED_ITEM_SIZE), limit),
            outstanding_credit: INITIAL_CREDIT,
        }
    }

    pub(crate) fn on_receive(&mut self, bytes: usize) {
        if self.item_size.is_none() {
            self.largest = self.largest.max(u32::try_from(bytes).unwrap_or(u32::MAX));
        }
    }

    pub(crate) fn start(&mut self) -> Option<u32> {
        self.grant()
    }

    pub(crate) fn consume_item(&mut self) -> Option<u32> {
        self.outstanding_credit = self.outstanding_credit.saturating_sub(1);
        if self.item_size.is_none() && self.largest > 0 {
            self.window = Self::fit(self.largest, self.limit);
        }

        if self.outstanding_credit > self.window / 2 {
            return None;
        }

        self.grant()
    }

    fn grant(&mut self) -> Option<u32> {
        let count = self
            .window
            .checked_sub(self.outstanding_credit)
            .filter(|&count| count > 0)?;

        self.outstanding_credit = self.window;

        Some(count)
    }

    fn fit(size: u32, most: u32) -> u32 {
        (STREAM_BUDGET / size.max(1)).clamp(1, most)
    }
}

#[cfg(test)]
#[path = "tests/window.rs"]
mod tests;
