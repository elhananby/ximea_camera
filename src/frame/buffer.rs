use super::types::ArcFrame;
use std::collections::VecDeque;

pub struct FrameBuffer {
    buffer: VecDeque<ArcFrame>,
    capacity: usize,
}

impl FrameBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, frame: ArcFrame) {
        if self.buffer.len() == self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(frame);
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ArcFrame> {
        self.buffer.iter()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}