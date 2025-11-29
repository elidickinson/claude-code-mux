use bytes::Bytes;
use futures::stream::Stream;
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

/// SSE event from provider
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

impl SseEvent {
    /// Format as SSE output for client
    pub fn to_sse_string(&self) -> String {
        let mut output = String::new();

        if let Some(ref event_type) = self.event {
            output.push_str(&format!("event: {}\n", event_type));
        }

        output.push_str(&format!("data: {}\n\n", self.data));
        output
    }
}

/// Parse SSE events from a byte stream
pub fn parse_sse_events(input: &str) -> Vec<SseEvent> {
    let mut events = Vec::new();
    let mut current_event: Option<String> = None;
    let mut current_data = String::new();

    for line in input.lines() {
        if line.is_empty() {
            // Empty line marks end of event
            if !current_data.is_empty() {
                events.push(SseEvent {
                    event: current_event.take(),
                    data: current_data.clone(),
                });
                current_data.clear();
            }
        } else if let Some(data) = line.strip_prefix("data: ") {
            if !current_data.is_empty() {
                current_data.push('\n');
            }
            current_data.push_str(data);
        } else if let Some(event) = line.strip_prefix("event: ") {
            current_event = Some(event.to_string());
        }
        // Ignore other fields like "id:", "retry:", etc.
    }

    // Handle case where stream doesn't end with empty line
    if !current_data.is_empty() {
        events.push(SseEvent {
            event: current_event,
            data: current_data,
        });
    }

    events
}

/// Stream adapter that converts a reqwest Response stream into SSE events
#[pin_project]
pub struct SseStream<S> {
    #[pin]
    inner: S,
    buffer: String,
    /// Queue of parsed events waiting to be emitted
    event_queue: std::collections::VecDeque<SseEvent>,
}

impl<S> SseStream<S> {
    pub fn new(stream: S) -> Self {
        Self {
            inner: stream,
            buffer: String::new(),
            event_queue: std::collections::VecDeque::new(),
        }
    }
}

impl<S> Stream for SseStream<S>
where
    S: Stream<Item = Result<Bytes, reqwest::Error>>,
{
    type Item = Result<SseEvent, reqwest::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        // First, check if we have queued events to emit
        if let Some(event) = this.event_queue.pop_front() {
            return Poll::Ready(Some(Ok(event)));
        }

        // Poll the inner stream for new data
        match this.inner.poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                // Add new bytes to buffer
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    this.buffer.push_str(text);

                    // Try to parse complete events from buffer
                    // Note: We only clear buffer up to the last complete event
                    if let Some(last_event_end) = this.buffer.rfind("\n\n") {
                        let complete_portion = &this.buffer[..last_event_end + 2];
                        let events = parse_sse_events(complete_portion);

                        // Add all parsed events to queue
                        for event in events {
                            this.event_queue.push_back(event);
                        }

                        // Keep only the incomplete portion in buffer
                        *this.buffer = this.buffer[last_event_end + 2..].to_string();

                        // Return the first queued event if available
                        if let Some(event) = this.event_queue.pop_front() {
                            return Poll::Ready(Some(Ok(event)));
                        }
                    }
                }

                // If no complete event yet, continue polling
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => {
                // Stream ended - check if buffer has remaining data
                if !this.buffer.is_empty() {
                    let events = parse_sse_events(this.buffer);
                    *this.buffer = String::new();

                    // Add all parsed events to queue
                    for event in events {
                        this.event_queue.push_back(event);
                    }
                }

                // Return next queued event, or None if queue is empty
                if let Some(event) = this.event_queue.pop_front() {
                    return Poll::Ready(Some(Ok(event)));
                }

                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_single_event() {
        let input = "event: message\ndata: {\"test\":\"value\"}\n\n";
        let events = parse_sse_events(input);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.as_deref(), Some("message"));
        assert_eq!(events[0].data, "{\"test\":\"value\"}");
    }

    #[test]
    fn test_parse_sse_multiple_events() {
        let input = "event: start\ndata: {\"a\":1}\n\nevent: delta\ndata: {\"b\":2}\n\n";
        let events = parse_sse_events(input);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event.as_deref(), Some("start"));
        assert_eq!(events[1].event.as_deref(), Some("delta"));
    }

    #[test]
    fn test_parse_sse_no_event_type() {
        let input = "data: plain data\n\n";
        let events = parse_sse_events(input);

        assert_eq!(events.len(), 1);
        assert!(events[0].event.is_none());
        assert_eq!(events[0].data, "plain data");
    }
}
