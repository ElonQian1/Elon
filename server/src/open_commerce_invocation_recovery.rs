use std::{sync::Arc, time::Duration};

use crate::{store::Store, types::AppState};

pub(crate) fn recover_interrupted_invocations(store: &Store) -> usize {
    let recovered = match store.recover_interrupted_open_commerce_invocations() {
        Ok(count) => count,
        Err(error) => {
            tracing::warn!(%error, "开放商业调用启动恢复失败");
            return 0;
        }
    };
    if recovered > 0 {
        tracing::warn!(
            count = recovered,
            "开放商业调用因服务器重启失败关闭，Grant 预算预留已按记录回收"
        );
    }
    recovered
}

pub(crate) fn reconcile_expired_invocations(store: &Store) -> usize {
    let recovered = match store.reconcile_expired_open_commerce_invocations() {
        Ok(count) => count,
        Err(error) => {
            tracing::warn!(%error, "开放商业过期调用回收失败");
            return 0;
        }
    };
    if recovered > 0 {
        tracing::warn!(
            count = recovered,
            "开放商业调用租约过期，调用已失败关闭并回收 Grant 预算预留"
        );
    }
    recovered
}

pub(crate) fn spawn_expired_invocation_reconciler(state: Arc<AppState>) {
    let interval_secs = std::env::var("OPEN_COMMERCE_INVOCATION_RECONCILE_SECS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(30)
        .max(5);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            ticker.tick().await;
            reconcile_expired_invocations(&state.store);
        }
    });
}
