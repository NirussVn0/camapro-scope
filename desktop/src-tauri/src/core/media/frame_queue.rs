use std::collections::VecDeque;

/// Bounded queue holding at most `capacity` frames.
/// Drops oldest frames when capacity is exceeded to prevent queue growth and backpressure lag.
pub struct BoundedFrameQueue<T> {
    queue: VecDeque<T>,
    capacity: usize,
    max_frame_bytes: usize,
    dropped_count: usize,
}

impl<T: AsRef<[u8]>> BoundedFrameQueue<T> {
    pub fn new(capacity: usize, max_frame_bytes: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(capacity),
            capacity,
            max_frame_bytes,
            dropped_count: 0,
        }
    }

    pub fn push(&mut self, frame: T) -> Result<(), &'static str> {
        if frame.as_ref().len() > self.max_frame_bytes {
            return Err("oversized_frame");
        }

        while self.queue.len() >= self.capacity {
            self.queue.pop_front();
            self.dropped_count += 1;
        }

        self.queue.push_back(frame);
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn dropped_count(&self) -> usize {
        self.dropped_count
    }
}
