use std::sync::Arc;

use rig_core::{completion::GetTokenUsage, streaming::StreamingCompletionResponse};
use tokio::sync::Mutex;

/// Type-erased pause/resume/cancel control for an in-flight streaming
/// generation, so callers (the engine, the UI) don't need to know the
/// concrete provider response type `R` behind a running stream.
///
/// `rig_core::streaming::StreamingCompletionResponse::{pause,resume,cancel,is_paused}`
/// all take `&self`, but actually consuming the stream (`StreamExt::next`)
/// needs `&mut self`. We reconcile this by sharing the response behind
/// `Arc<Mutex<_>>`: the poller locks briefly around each `.next().await`, and
/// control calls lock briefly too. A pause/cancel issued while a chunk is in
/// flight simply waits for that chunk to arrive (or the stream to end) before
/// applying — there's no way to interrupt an in-flight network read any
/// faster than that with the APIs rig-core exposes.
#[async_trait::async_trait]
pub trait StreamControl: Send + Sync {
    async fn pause(&self);
    async fn resume(&self);
    async fn cancel(&self);
    async fn is_paused(&self) -> bool;
}

pub(crate) struct ResponseControl<R>(pub Arc<Mutex<StreamingCompletionResponse<R>>>)
where
    R: Clone + Unpin + GetTokenUsage;

#[async_trait::async_trait]
impl<R> StreamControl for ResponseControl<R>
where
    R: Clone + Unpin + GetTokenUsage + Send + 'static,
{
    async fn pause(&self) {
        self.0.lock().await.pause();
    }

    async fn resume(&self) {
        self.0.lock().await.resume();
    }

    async fn cancel(&self) {
        self.0.lock().await.cancel();
    }

    async fn is_paused(&self) -> bool {
        self.0.lock().await.is_paused()
    }
}
