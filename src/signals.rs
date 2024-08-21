use anyhow::Result;
use tokio::signal::{
    self,
    unix::{signal, SignalKind},
};
use tokio::sync::broadcast;

pub async fn shutdown_channel() -> Result<broadcast::Receiver<()>> {
    let (tx, rx) = broadcast::channel(4);

    tokio::spawn(async move {
        shutdown_signal().await;
        tx.send(())
            .expect("Failed to send shutdown signal on channel")
    });

    Ok(rx)
}

// based on:
// https://github.com/davidpdrsn/realworld-axum-sqlx/blob/main/src/http/mod.rs
pub async fn shutdown_signal() {
    let sig_int = async {
        signal::ctrl_c()
            .await
            .expect("Failed to register SIGINT handler");
    };

    #[cfg(unix)]
    let sig_term = async {
        signal(SignalKind::terminate())
            .expect("Failed to register SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sig_term = std::future::pending::<()>();

    #[cfg(unix)]
    let sig_quit = async {
        signal(SignalKind::quit())
            .expect("Failed to register SIGQUIT handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sig_quit = std::future::pending::<()>();

    #[cfg(unix)]
    let sig_hup = async {
        signal(SignalKind::hangup())
            .expect("Failed to register SIGHUP handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sig_hup = std::future::pending::<()>();

    tokio::select! {
        _ = sig_int => {},
        _ = sig_term => {},
        _ = sig_quit => {},
        _ = sig_hup => {},
    }
}
