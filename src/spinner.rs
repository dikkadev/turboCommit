use std::{
    io::{stdout, Write},
    time::Instant,
};

use atty::Stream;
use colored::Colorize;
use tokio::{
    sync::oneshot,
    task::JoinHandle,
    time::{interval, Duration},
};

/// Simple clock-emoji spinner that overwrites the same line.
pub struct Spinner {
    stop_tx: Option<oneshot::Sender<()>>,
    handle: Option<JoinHandle<()>>,
}

impl Spinner {
    pub fn start(label: String) -> Option<Self> {
        if !atty::is(Stream::Stdout) {
            return None;
        }
        if !terminal_supports_emoji::supports_emoji(terminal_supports_emoji::Stream::Stdout) {
            return None;
        }

        let (tx, mut rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            const CLOCKS: [&str; 12] = [
                "🕛", "🕐", "🕑", "🕒", "🕓", "🕔", "🕕", "🕖", "🕗", "🕘", "🕙", "🕚",
            ];
            let mut ticker = interval(Duration::from_millis(100));
            let mut idx = 0usize;
            let mut stdout = stdout();
            let start = Instant::now();

            loop {
                tokio::select! {
                    _ = &mut rx => {
                        let _ = write!(stdout, "\r\x1b[K");
                        let _ = stdout.flush();
                        break;
                    }
                    _ = ticker.tick() => {
                        let emoji = CLOCKS[idx % CLOCKS.len()];
                        let elapsed = start.elapsed().as_secs_f32();
                        let timer_text = format!("({:.1}s)", elapsed).bright_black();
                        let display = format!("{emoji} {timer_text} {label}");
                        let _ = write!(stdout, "\r{display}\x1b[K");
                        let _ = stdout.flush();
                        idx = (idx + 1) % CLOCKS.len();
                    }
                }
            }
        });

        Some(Self {
            stop_tx: Some(tx),
            handle: Some(handle),
        })
    }

    pub async fn stop(mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.await;
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }
}
