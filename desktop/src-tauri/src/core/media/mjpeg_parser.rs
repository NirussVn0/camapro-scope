/// Incremental parser for multipart/x-mixed-replace MJPEG streams.
/// Bounded memory buffer protects against slow or malicious emitters.
pub struct MjpegStreamParser {
    buffer: Vec<u8>,
    max_buffer_bytes: usize,
}

impl MjpegStreamParser {
    pub fn new(max_buffer_bytes: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_buffer_bytes,
        }
    }

    pub fn feed(&mut self, chunk: &[u8]) -> Result<Vec<Vec<u8>>, &'static str> {
        if self.buffer.len() + chunk.len() > self.max_buffer_bytes {
            return Err("buffer_overflow");
        }

        self.buffer.extend_from_slice(chunk);
        let mut frames = Vec::new();

        while let Some(header_sep) = find_subsequence(&self.buffer, b"\r\n\r\n") {
            let header_slice = &self.buffer[..header_sep];
            let header_str = String::from_utf8_lossy(header_slice);

            // Parse Content-Length
            let content_len = match parse_content_length(&header_str) {
                Some(len) => len,
                None => {
                    // Skip malformed header
                    self.buffer.drain(..header_sep + 4);
                    continue;
                }
            };

            let body_start = header_sep + 4;
            let body_end = body_start + content_len;

            if self.buffer.len() < body_end {
                // Incomplete frame, wait for more data
                break;
            }

            let frame_data = self.buffer[body_start..body_end].to_vec();
            frames.push(frame_data);

            // Drain up to body_end plus any trailing \r\n
            let mut drain_end = body_end;
            if self.buffer.len() >= drain_end + 2
                && &self.buffer[drain_end..drain_end + 2] == b"\r\n"
            {
                drain_end += 2;
            }
            self.buffer.drain(..drain_end);
        }

        Ok(frames)
    }
}

fn parse_content_length(headers: &str) -> Option<usize> {
    for line in headers.lines() {
        let trimmed = line.trim();
        if trimmed.to_ascii_lowercase().starts_with("content-length:") {
            let val = trimmed["content-length:".len()..].trim();
            return val.parse::<usize>().ok();
        }
    }
    None
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
