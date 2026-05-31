//! Shared turn execution. Single source of truth for spawn-drain-cancel.

use crate::agent::agent::{Agent, TurnEvent};
use crate::agent::loop_::is_tool_loop_cancelled;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;
use zeroclaw_api::model_provider::ConversationMessage;

pub enum TurnOutcome {
    Completed {
        text: String,
        messages: Vec<ConversationMessage>,
    },
    Cancelled {
        partial_text: String,
        /// `true` when the drain self-cancelled on idle-stall (no events for
        /// [`DRAIN_IDLE_TIMEOUT`]); `false` when an external token fire
        /// (client RPC, reaper, session removal) reached the drain. Lets the
        /// caller record the right [`crate::rpc::session::CancelCause`].
        idle_stall: bool,
    },
}

#[derive(Debug)]
pub enum TurnError {
    Panicked(String),
    AgentError(String),
}

impl std::fmt::Display for TurnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Panicked(msg) => write!(f, "Turn task panicked: {msg}"),
            Self::AgentError(msg) => write!(f, "Agent turn failed: {msg}"),
        }
    }
}

impl std::error::Error for TurnError {}

/// Attribution fields attached to the tracing span for the duration of a turn.
/// All fields appear on every `record!()` emitted inside the turn.
#[derive(Clone, Default)]
pub struct TurnAttribution {
    pub session_key: Option<String>,
    pub agent_alias: String,
    pub model_provider: String,
    pub model: String,
    pub channel: &'static str,
}

pub async fn execute_turn<F, Fut>(
    agent: Arc<Mutex<Agent>>,
    prompt: String,
    cancel: CancellationToken,
    attribution: TurnAttribution,
    on_event: F,
) -> Result<TurnOutcome, TurnError>
where
    F: Fn(TurnEvent) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = ()> + Send,
{
    let (event_tx, mut event_rx) = mpsc::channel::<TurnEvent>(64);
    let cancel_clone = cancel.clone();
    let session_key = attribution.session_key.clone();

    let turn_handle = zeroclaw_spawn::spawn!(async move {
        let mut guard = agent.lock().await;
        let sk = attribution.session_key.clone();
        crate::agent::loop_::scope_session_key(attribution.session_key, async move {
            use ::zeroclaw_log::Instrument as _;
            let span = ::zeroclaw_log::info_span!(
                target: "zeroclaw_log_internal_scope",
                "zeroclaw_scope",
                session_key = %sk.as_deref().unwrap_or(""),
                agent_alias = %attribution.agent_alias,
                model_provider = %attribution.model_provider,
                model = %attribution.model,
                channel = %attribution.channel,
            );
            guard
                .turn_streamed(&prompt, event_tx, Some(cancel_clone))
                .instrument(span)
                .await
        })
        .await
    });

    let mut accumulated_text = String::new();

    // Drive the turn by draining its event channel, but never let a turn task
    // wedged inside a non-cancellable tool call (shell, HTTP, a stalled provider
    // stream) hold the dispatch path hostage. The drain exits on channel close,
    // explicit cancel, OR an idle-stall bound; the latter two return Cancelled
    // and the in-flight task is aborted on drop.
    let drain =
        drain_until_done_or_cancelled(&mut event_rx, &cancel, &mut accumulated_text, &on_event)
            .await;
    let _ = session_key; // consumed above

    match drain {
        DrainOutcome::Completed => match turn_handle
            .await
            .map_err(|e| TurnError::Panicked(format!("{e}")))?
        {
            Ok((text, messages)) => Ok(TurnOutcome::Completed { text, messages }),
            Err(e) if is_tool_loop_cancelled(&e) => Ok(TurnOutcome::Cancelled {
                partial_text: accumulated_text,
                idle_stall: false,
            }),
            Err(e) => Err(TurnError::AgentError(format!("{e}"))),
        },
        DrainOutcome::ExplicitCancel | DrainOutcome::IdleStall => {
            turn_handle.abort();
            Ok(TurnOutcome::Cancelled {
                partial_text: accumulated_text,
                idle_stall: matches!(drain, DrainOutcome::IdleStall),
            })
        }
    }
}

/// Why [`drain_until_done_or_cancelled`] returned. `IdleStall` is a self-fired
/// cancel (no events for [`DRAIN_IDLE_TIMEOUT`]); `ExplicitCancel` is an
/// outside fire that reached the drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DrainOutcome {
    Completed,
    ExplicitCancel,
    IdleStall,
}

/// Drain `event_rx` until the turn finishes, the cancel token fires, or the
/// stream stalls past [`DRAIN_IDLE_TIMEOUT`]. On idle, fires `cancel` itself
/// so downstream sees a unified cancel shape. Chunk deltas accumulate in
/// `accumulated` so partial text survives a cancel.
const DRAIN_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

async fn drain_until_done_or_cancelled<F, Fut>(
    event_rx: &mut mpsc::Receiver<TurnEvent>,
    cancel: &CancellationToken,
    accumulated: &mut String,
    on_event: &F,
) -> DrainOutcome
where
    F: Fn(TurnEvent) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    loop {
        if cancel.is_cancelled() {
            return DrainOutcome::ExplicitCancel;
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return DrainOutcome::ExplicitCancel,
            maybe_event = event_rx.recv() => {
                match maybe_event {
                    Some(event) => {
                        if let TurnEvent::Chunk { ref delta } = event {
                            accumulated.push_str(delta);
                        }
                        on_event(event).await;
                    }
                    None => return DrainOutcome::Completed,
                }
            }
            _ = tokio::time::sleep(DRAIN_IDLE_TIMEOUT) => {
                cancel.cancel();
                return DrainOutcome::IdleStall;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn noop(_e: TurnEvent) -> std::future::Ready<()> {
        std::future::ready(())
    }

    #[tokio::test]
    async fn drain_must_not_hang_when_no_events_and_no_cancel() {
        let (sender_kept_alive, mut event_rx) = mpsc::channel::<TurnEvent>(8);
        let cancel = CancellationToken::new();
        let mut acc = String::new();

        let elapsed = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            drain_until_done_or_cancelled(&mut event_rx, &cancel, &mut acc, &noop),
        )
        .await;

        drop(sender_kept_alive);

        elapsed.expect(
            "drain must return when no events arrive and no cancel fires; \
             an unbounded idle wait is the TUI 'working' freeze. The fix must \
             cap the idle-event window at a value substantially below 5s in \
             default test configuration; the production threshold may be \
             larger but must always be finite.",
        );
    }

    #[tokio::test]
    async fn drain_must_still_accumulate_chunks_when_events_arrive_steadily() {
        let (tx, mut rx) = mpsc::channel::<TurnEvent>(8);
        let cancel = CancellationToken::new();
        let mut acc = String::new();

        let sender = tokio::spawn(async move {
            for delta in ["he", "llo", " ", "world"] {
                let _ = tx
                    .send(TurnEvent::Chunk {
                        delta: delta.to_string(),
                    })
                    .await;
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
            }
        });

        let cancelled = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            drain_until_done_or_cancelled(&mut rx, &cancel, &mut acc, &noop),
        )
        .await
        .expect("drain must terminate after the sender drops");

        sender.await.unwrap();
        assert_eq!(
            cancelled,
            DrainOutcome::Completed,
            "channel closure is not a cancel; drain returned the wrong verdict"
        );
        assert_eq!(
            acc, "hello world",
            "drain dropped chunks instead of accumulating them; a fix that \
             short-circuits with too-aggressive an idle window (e.g. <250ms) \
             would corrupt legitimate streaming turns. The production idle \
             window must sit comfortably between the inter-chunk gap of a \
             healthy stream (~hundreds of ms) and the user-perceptible hang \
             threshold (~seconds)."
        );
    }
}
