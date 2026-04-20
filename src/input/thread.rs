use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use crossterm::event::{Event, KeyEvent, KeyEventKind};

/// An input event with a precise timestamp taken at read() time.
#[derive(Debug, Clone)]
pub struct TimestampedEvent {
    pub instant: Instant,
    pub event: InputEvent,
}

/// Input events sent from the input thread to the game thread.
#[derive(Debug, Clone)]
pub enum InputEvent {
    Key(KeyEvent),
    Resize(u16, u16),
}

/// Spawn the dedicated input polling thread.
///
/// Uses poll(10ms) + read() to avoid blocking indefinitely.
/// Checks the shutdown flag between polls for clean exit.
///
/// Latency budget: The 10ms poll timeout adds up to 10ms before the event
/// reaches the game thread. Combined with the game tick interval (~8.3ms at
/// 120Hz), worst-case delivery delay is ~18ms. This does NOT affect timestamp
/// accuracy (timestamp is taken at read() time, not delivery time).
pub fn spawn_input_thread(
    tx: Sender<TimestampedEvent>,
    shutdown: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let poll_timeout = Duration::from_millis(10);
        while !shutdown.load(Ordering::Relaxed) {
            if crossterm::event::poll(poll_timeout).unwrap_or(false) {
                match crossterm::event::read() {
                    Ok(Event::Key(key_event)) => {
                        if key_event.kind == KeyEventKind::Press {
                            let ts = Instant::now();
                            if tx
                                .send(TimestampedEvent {
                                    instant: ts,
                                    event: InputEvent::Key(key_event),
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                    Ok(Event::Resize(cols, rows)) => {
                        let _ = tx.send(TimestampedEvent {
                            instant: Instant::now(),
                            event: InputEvent::Resize(cols, rows),
                        });
                    }
                    _ => {}
                }
            }
        }
    })
}
