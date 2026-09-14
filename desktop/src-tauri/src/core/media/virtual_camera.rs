//! G3.1 virtual camera sink core: consumes decoded frames from
//! [`BoundedFrameQueue`] and writes them to an injectable [`FrameSink`].
//! Synthetic slice only — no kernel module, no GStreamer.

use std::sync::Arc;

use super::frame_queue::BoundedFrameQueue;

/// Negotiated output parameters; exact match required, mismatches rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoFormat {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub pixel_format: &'static str,
}

/// Typed, visible failure modes for the virtual camera output path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VirtualCameraError {
    NoSink,
    SinkBusy,
    FormatMismatch(VideoFormat),
}

/// Injectable sink target. Synchronous by design; the pump is polled.
pub trait FrameSink: Send + Sync {
    fn write(&self, frame: &[u8]) -> Result<(), VirtualCameraError>;
}

pub struct VirtualCameraSink {
    format: VideoFormat,
    queue: BoundedFrameQueue<Vec<u8>>,
    sink: Option<Arc<dyn FrameSink>>,
    started: bool,
}

impl VirtualCameraSink {
    pub fn new(format: VideoFormat, queue_capacity: usize, max_frame_bytes: usize) -> Self {
        Self {
            format,
            queue: BoundedFrameQueue::new(queue_capacity, max_frame_bytes),
            sink: None,
            started: false,
        }
    }

    /// Accept only an exact format match; mismatches are rejected explicitly.
    pub fn negotiate(&mut self, format: VideoFormat) -> Result<(), VirtualCameraError> {
        if format == self.format {
            Ok(())
        } else {
            Err(VirtualCameraError::FormatMismatch(self.format.clone()))
        }
    }

    /// Attach (or replace) the consumer. Only one sink may be attached;
    /// a second attach while active is `SinkBusy`.
    pub fn attach_sink(&mut self, sink: Arc<dyn FrameSink>) -> Result<(), VirtualCameraError> {
        if self.started {
            return Err(VirtualCameraError::SinkBusy);
        }
        self.sink = Some(sink);
        Ok(())
    }

    pub fn start(&mut self) -> Result<(), VirtualCameraError> {
        if self.sink.is_none() {
            return Err(VirtualCameraError::NoSink);
        }
        self.started = true;
        Ok(())
    }

    /// Enqueue a decoded frame. Oldest frames are dropped when the bounded
    /// queue is full; the drop count is visible via [`Self::dropped_count`].
    /// Never blocks and never fails due to a slow sink — backpressure is
    /// absorbed by dropping stale frames.
    pub fn submit(&mut self, frame: Vec<u8>) -> Result<(), &'static str> {
        self.queue.push(frame)
    }

    /// Polling pump: drain the queue into the sink. Returns the number of
    /// frames written, or a typed error if no sink is attached or the sink
    /// reports busy. No threads, no blocking.
    pub fn pump(&mut self) -> Result<usize, VirtualCameraError> {
        if !self.started {
            return Err(VirtualCameraError::NoSink);
        }
        let sink = match self.sink.as_ref() {
            Some(s) => s.clone(),
            None => return Err(VirtualCameraError::NoSink),
        };
        let mut written = 0;
        while let Some(frame) = self.queue.pop() {
            sink.write(&frame)?;
            written += 1;
        }
        Ok(written)
    }

    pub fn dropped_count(&self) -> usize {
        self.queue.dropped_count()
    }

    pub fn is_active(&self) -> bool {
        self.started
    }

    /// Always releases the sink; nothing stays "running" after teardown.
    pub fn stop(&mut self) {
        self.started = false;
        self.sink = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FMT: VideoFormat = VideoFormat {
        width: 640,
        height: 480,
        fps: 30,
        pixel_format: "MJPG",
    };

    struct FakeSink {
        frames: std::sync::Mutex<Vec<Vec<u8>>>,
        busy: std::sync::atomic::AtomicBool,
    }

    impl FakeSink {
        fn new() -> Self {
            Self {
                frames: std::sync::Mutex::new(Vec::new()),
                busy: std::sync::atomic::AtomicBool::new(false),
            }
        }
        fn captured(&self) -> Vec<Vec<u8>> {
            self.frames.lock().unwrap().clone()
        }
    }

    impl FrameSink for FakeSink {
        fn write(&self, frame: &[u8]) -> Result<(), VirtualCameraError> {
            if self.busy.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(VirtualCameraError::SinkBusy);
            }
            self.frames.lock().unwrap().push(frame.to_vec());
            Ok(())
        }
    }

    // Goal 1: frames reach the sink in order while a consumer is attached.
    #[test]
    fn frames_reach_sink_in_order() {
        let mut cam = VirtualCameraSink::new(FMT.clone(), 4, 1024);
        let sink = Arc::new(FakeSink::new());
        cam.attach_sink(sink.clone()).unwrap();
        cam.start().unwrap();
        for i in 0..3u8 {
            cam.submit(vec![i; 8]).unwrap();
        }
        assert_eq!(cam.pump().unwrap(), 3);
        let got = sink.captured();
        assert_eq!(got.len(), 3);
        for (i, f) in got.iter().enumerate() {
            assert_eq!(f[0], i as u8);
        }
    }

    // Goal 2: bounded queue drops stale frames; count visible; producer
    // (submit) never blocks or fails due to a stalled sink.
    #[test]
    fn stalled_sink_drops_stale_frames_counted() {
        let mut cam = VirtualCameraSink::new(FMT.clone(), 2, 1024);
        let sink = Arc::new(FakeSink::new());
        cam.attach_sink(sink.clone()).unwrap();
        cam.start().unwrap();
        // Sink is stalled (busy): pump surfaces busy, queue keeps dropping.
        sink.busy.store(true, std::sync::atomic::Ordering::SeqCst);
        for i in 0..5u8 {
            cam.submit(vec![i; 8]).unwrap();
        }
        assert_eq!(cam.dropped_count(), 3);
        assert_eq!(cam.pump(), Err(VirtualCameraError::SinkBusy));
        // Stale frames were dropped, not accumulated: capacity never exceeded.
        assert_eq!(sink.captured().len(), 0);
    }

    // Goal 3: missing sink is a typed error; stop() releases the sink.
    #[test]
    fn missing_sink_typed_error_and_stop_releases() {
        let mut cam = VirtualCameraSink::new(FMT.clone(), 2, 1024);
        assert_eq!(cam.pump(), Err(VirtualCameraError::NoSink));
        let sink = Arc::new(FakeSink::new());
        cam.attach_sink(sink.clone()).unwrap();
        cam.start().unwrap();
        assert!(cam.is_active());
        cam.submit(vec![0; 8]).unwrap();
        cam.stop();
        assert!(!cam.is_active());
        // Sink released: pump after stop reports NoSink, nothing further sent.
        assert_eq!(cam.pump(), Err(VirtualCameraError::NoSink));
    }

    // Goal 3 (variant): a closed sink also surfaces a typed error.
    struct ClosedSink;
    impl FrameSink for ClosedSink {
        fn write(&self, _frame: &[u8]) -> Result<(), VirtualCameraError> {
            Err(VirtualCameraError::SinkBusy)
        }
    }

    #[test]
    fn closed_sink_surfaces_typed_error() {
        let mut cam = VirtualCameraSink::new(FMT.clone(), 2, 1024);
        cam.attach_sink(Arc::new(ClosedSink)).unwrap();
        cam.start().unwrap();
        cam.submit(vec![1; 8]).unwrap();
        assert_eq!(cam.pump(), Err(VirtualCameraError::SinkBusy));
    }

    // Goal 4: format negotiation — explicit match or typed mismatch.
    #[test]
    fn negotiation_accepts_match_rejects_mismatch() {
        let mut cam = VirtualCameraSink::new(FMT.clone(), 2, 1024);
        assert!(cam.negotiate(FMT.clone()).is_ok());
        let other = VideoFormat {
            width: 1280,
            height: 720,
            fps: 60,
            pixel_format: "YUY2",
        };
        assert_eq!(
            cam.negotiate(other),
            Err(VirtualCameraError::FormatMismatch(FMT.clone()))
        );
    }
}
