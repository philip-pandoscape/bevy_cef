use bevy::prelude::Deref;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

/// Maximum delay between message loop iterations (~30fps).
/// Following CEF's official cefclient pattern (`kMaxTimerDelay = 1000/30`).
const MAX_TIMER_DELAY_MS: u64 = 1000 / 30;

#[repr(transparent)]
#[derive(Debug, Deref)]
pub struct MessageLoopWorkingReceiver(pub Receiver<MessageLoopTimer>);

#[derive(Debug)]
pub struct MessageLoopTimer(Instant);

impl MessageLoopTimer {
    pub fn new(delay_ms: i64) -> Self {
        let fire_time = if delay_ms <= 0 {
            Instant::now()
        } else {
            Instant::now() + Duration::from_millis(delay_ms as u64)
        };
        Self(fire_time)
    }

    #[inline]
    pub fn is_finished(&self) -> bool {
        self.0 <= Instant::now()
    }
}

#[derive(Debug, Deref)]
pub struct MessageLoopWorkingMaxDelayTimer(MessageLoopTimer);

impl Default for MessageLoopWorkingMaxDelayTimer {
    fn default() -> Self {
        Self(MessageLoopTimer::new(MAX_TIMER_DELAY_MS as i64))
    }
}

/// CEF asking for its message loop to be worked at once, as something an embedder can wait on.
///
/// bevy_cef works the loop once a frame, from the `Main` schedule. Much of what CEF does for a
/// frame of a page (its own compositor's part, the hand-over of a painted picture) runs on the UI
/// thread, which is the app's main thread, so between two frames it stands still. An embedder
/// whose main thread waits for something else (Bevy's waits for its render thread at the end of
/// every frame) can wait on this as well, and call `cef::do_message_loop_work()` on that thread
/// each time it is signalled: CEF then gets on with its frame in time the app was not using.
///
/// One signal stands for any number of requests since the last one taken.
pub fn work_wanted() -> async_channel::Receiver<()> {
    work_wanted_channel().1.clone()
}

fn work_wanted_channel() -> &'static (async_channel::Sender<()>, async_channel::Receiver<()>) {
    static CHANNEL: std::sync::OnceLock<(async_channel::Sender<()>, async_channel::Receiver<()>)> =
        std::sync::OnceLock::new();
    CHANNEL.get_or_init(|| async_channel::bounded(1))
}

/// CEF asked for work with no delay (from any thread).
pub(crate) fn want_work() {
    // (Full: a signal is waiting already.)
    let _ = work_wanted_channel().0.try_send(());
}
