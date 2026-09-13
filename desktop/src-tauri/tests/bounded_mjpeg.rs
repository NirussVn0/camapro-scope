use camapro_scope_lib::core::media::{BoundedFrameQueue, MjpegStreamParser};

#[test]
fn bounded_queue_drops_oldest_frame_under_slow_consumer() {
    let mut queue = BoundedFrameQueue::new(2, 1024 * 1024);

    queue.push(vec![1, 1, 1]).unwrap();
    queue.push(vec![2, 2, 2]).unwrap();
    assert_eq!(queue.len(), 2);
    assert_eq!(queue.dropped_count(), 0);

    // Push 3rd frame: must drop frame 1
    queue.push(vec![3, 3, 3]).unwrap();
    assert_eq!(queue.len(), 2);
    assert_eq!(queue.dropped_count(), 1);

    assert_eq!(queue.pop(), Some(vec![2, 2, 2]));
    assert_eq!(queue.pop(), Some(vec![3, 3, 3]));
    assert_eq!(queue.pop(), None);
}

#[test]
fn bounded_queue_rejects_oversized_frame() {
    let mut queue = BoundedFrameQueue::new(2, 100);
    let oversized = vec![0u8; 101];
    assert!(queue.push(oversized).is_err());
    assert_eq!(queue.len(), 0);
}

#[test]
fn mjpeg_parser_extracts_complete_frames_and_bounds_memory() {
    let mut parser = MjpegStreamParser::new(1024 * 1024);

    let part1 = b"--frame\r\nContent-Type: image/jpeg\r\nContent-Length: 4\r\n\r\nJPEG\r\n";
    let frames = parser.feed(part1).expect("feed must succeed");
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0], b"JPEG");

    // Partial chunk feed
    let part2_chunk1 = b"--frame\r\nContent-Type: image/jpeg\r\nContent-Length: 5\r\n\r\nHE";
    let frames = parser.feed(part2_chunk1).expect("feed must succeed");
    assert_eq!(frames.len(), 0);

    let part2_chunk2 = b"LLO\r\n";
    let frames = parser.feed(part2_chunk2).expect("feed must succeed");
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0], b"HELLO");
}

#[test]
fn mjpeg_parser_rejects_oversized_buffer() {
    let mut parser = MjpegStreamParser::new(50);
    let oversized = vec![b'A'; 51];
    assert!(parser.feed(&oversized).is_err());
}
