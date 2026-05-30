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

    // Caller-side copy of the attribution fields. The spawned turn task moves
    // `attribution`, but the drain loop below runs in THIS task and emits its
    // own span-attributed record on the cancel-mid-drain path, so it needs the
    // same fields to reconstruct the `zeroclaw_scope` span.
    let attr_session_key = attribution.session_key.clone();
    let attr_agent_alias = attribution.agent_alias.clone();
    let attr_model_provider = attribution.model_provider.clone();
    let attr_model = attribution.model.clone();
    let attr_channel = attribution.channel;

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
    // that is wedged inside a non-cancellable tool call (shell, HTTP, a stalled
    // provider stream) hold the dispatch path hostage. The event loop exits on
    // EITHER the channel closing (turn returned) OR the cancel token firing.
    // Without the cancel arm, a fired token that the in-flight tool does not
    // poll would leave `event_rx.recv()` awaiting forever, the turn task never
    // dropping `event_tx`, and the caller never reaching `emit_turn_complete`
    // — the exact TUI hang where `Esc` parks the client in `Cancelling` with no
    // terminal `TurnComplete` ever sent.
    let cancelled_mid_drain =
        drain_until_done_or_cancelled(&mut event_rx, &cancel, &mut accumulated_text, &on_event)
            .await;
    let _ = session_key; // consumed above

    if cancelled_mid_drain {
        // The token fired while the turn task may still be blocked in a tool
        // call that does not observe cancellation. Detach the task so it aborts
        // on drop (or unwinds when its next await point sees the token) and
        // return the verdict NOW. The dispatch path emits `TurnComplete` from
        // here, guaranteeing the client always exits the working state.
        //
        // Emit the observability marker for this exact site: before the
        // select-on-cancel arm existed, control would have parked on
        // `event_rx.recv()` here forever — the TUI freeze. Recording it under
        // the turn's attribution span turns every would-have-frozen turn into a
        // searchable, attributed event instead of a silent hang.
        {
            use ::zeroclaw_log::Instrument as _;
            let span = ::zeroclaw_log::info_span!(
                target: "zeroclaw_log_internal_scope",
                "zeroclaw_scope",
                session_key = %attr_session_key.as_deref().unwrap_or(""),
                agent_alias = %attr_agent_alias,
                model_provider = %attr_model_provider,
                model = %attr_model,
                channel = %attr_channel,
            );
            async {
                ::zeroclaw_log::record!(
                    DEBUG,
                    ::zeroclaw_log::Event::new(module_path!(), ::zeroclaw_log::Action::Cancel)
                        .with_category(::zeroclaw_log::EventCategory::Agent)
                        .with_attrs(::serde_json::json!({
                            "partial_text_len": accumulated_text.len(),
                        })),
                    "turn: cancel fired mid-drain; returning Cancelled and aborting detached turn task (pre-fix this site would have hung waiting on a wedged tool)"
                );
            }
            .instrument(span)
            .await;
        }
        turn_handle.abort();
        return Ok(TurnOutcome::Cancelled {
            partial_text: accumulated_text,
        });
    }

    match turn_handle
        .await
        .map_err(|e| TurnError::Panicked(format!("{e}")))?
    {
        Ok((text, messages)) => Ok(TurnOutcome::Completed { text, messages }),
        Err(e) if is_tool_loop_cancelled(&e) => Ok(TurnOutcome::Cancelled {
            partial_text: accumulated_text,
        }),
        Err(e) => Err(TurnError::AgentError(format!("{e}"))),
    }
}

/// Drain `event_rx`, forwarding each event to `on_event`, until EITHER the
/// channel closes (the turn task returned and dropped its sender) OR `cancel`
/// fires. Returns `true` when it stopped because cancel fired — the signal that
/// the turn task may be wedged and the caller must return the verdict without
/// awaiting it. Chunk deltas are appended to `accumulated` so a cancel verdict
/// can still carry whatever partial text streamed before the stop.
async fn drain_until_done_or_cancelled<F, Fut>(
    event_rx: &mut mpsc::Receiver<TurnEvent>,
    cancel: &CancellationToken,
    accumulated: &mut String,
    on_event: &F,
) -> bool
where
    F: Fn(TurnEvent) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    loop {
        if cancel.is_cancelled() {
            return true;
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => return true,
            maybe_event = event_rx.recv() => {
                match maybe_event {
                    Some(event) => {
                        if let TurnEvent::Chunk { ref delta } = event {
                            accumulated.push_str(delta);
                        }
                        on_event(event).await;
                    }
                    None => return false,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop(_e: TurnEvent) -> std::future::Ready<()> {
        std::future::ready(())
    }

    // The regression guard for the TUI hang: a turn task wedged inside a
    // non-cancellable tool never closes the event channel, so the drain must
    // return on the cancel token alone — promptly — rather than parking on
    // `recv()` forever. `tx` is held (never dropped) to model the wedge.
    #[tokio::test]
    async fn cancel_unblocks_drain_when_channel_never_closes() {
        let (tx, mut rx) = mpsc::channel::<TurnEvent>(8);
        let cancel = CancellationToken::new();
        let mut acc = String::new();

        let c2 = cancel.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            c2.cancel();
        });

        let cancelled = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            drain_until_done_or_cancelled(&mut rx, &cancel, &mut acc, &noop),
        )
        .await
        .expect("drain must return once cancel fires, not hang on a wedged channel");

        assert!(cancelled, "drain stopped because cancel fired");
        drop(tx); // keep the sender alive until here to prove the channel stayed open
    }

    // A token already cancelled before the first poll must short-circuit to a
    // cancelled verdict without consuming an event.
    #[tokio::test]
    async fn pre_cancelled_token_returns_immediately() {
        let (_tx, mut rx) = mpsc::channel::<TurnEvent>(8);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let mut acc = String::new();
        let cancelled = drain_until_done_or_cancelled(&mut rx, &cancel, &mut acc, &noop).await;
        assert!(cancelled);
    }

    // Normal completion: the turn task finishes and drops the sender. The drain
    // must observe the closed channel and report NOT-cancelled, having
    // accumulated the streamed chunk text.
    #[tokio::test]
    async fn closed_channel_returns_not_cancelled_and_accumulates() {
        let (tx, mut rx) = mpsc::channel::<TurnEvent>(8);
        let cancel = CancellationToken::new();
        let mut acc = String::new();

        tokio::spawn(async move {
            let _ = tx
                .send(TurnEvent::Chunk {
                    delta: "hello".to_string(),
                })
                .await;
            // sender dropped here -> channel closes
        });

        let cancelled = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            drain_until_done_or_cancelled(&mut rx, &cancel, &mut acc, &noop),
        )
        .await
        .expect("drain must return when the channel closes");

        assert!(!cancelled, "completion is not a cancel");
        assert_eq!(acc, "hello");
    }
}
