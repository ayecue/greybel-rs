use super::*;

pub fn grants(window: &mut ReceiveWindow, taken: u32, bytes_each: usize) -> Vec<u32> {
    let mut granted: Vec<u32> = window.start().into_iter().collect();

    for _ in 0..taken {
        window.on_receive(bytes_each);
        granted.extend(window.consume_item());
    }

    granted
}
