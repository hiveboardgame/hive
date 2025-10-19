use std::future::Future;

use tokio::{select, spawn};
use tokio_util::sync::CancellationToken;

pub fn spawn_abortable<F>(task: F, token: CancellationToken)
where
    F: Future<Output = ()> + Send + 'static,
{
    spawn(async move {
        select! {
            _ = token.cancelled() => {}
            _ = task => {}
        }
    });
}
