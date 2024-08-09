use anyhow::Result;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::broadcast;

// This could be a lot more generic and flexible, but will do for now

#[derive(Clone, Debug)]
pub enum Signal {
    Shutdown,
    Usr1,
    Usr2,
}

pub async fn listen() -> Result<broadcast::Receiver<Signal>> {
    let (tx, rx) = broadcast::channel(6);

    let mut int = signal(SignalKind::interrupt())?;
    let mut term = signal(SignalKind::terminate())?;
    let mut hup = signal(SignalKind::hangup())?;
    let mut quit = signal(SignalKind::quit())?;
    let mut usr1 = signal(SignalKind::user_defined1())?;
    let mut usr2 = signal(SignalKind::user_defined2())?;

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = int.recv() => {
                    tx.send(Signal::Shutdown).unwrap();
                    break;
                },
                _ = term.recv() => {
                    tx.send(Signal::Shutdown).unwrap();
                    break;
                },
                _ = hup.recv() => {
                    tx.send(Signal::Shutdown).unwrap();
                    break;
                },
                _ = quit.recv() => {
                    tx.send(Signal::Shutdown).unwrap();
                    break;
                },
                _ = usr1.recv() => {
                    tx.send(Signal::Usr1).unwrap();
                    continue;
                },
                _ = usr2.recv() => {
                    tx.send(Signal::Usr2).unwrap();
                    continue;
                },
            }
        }
    });
    Ok(rx)
}
