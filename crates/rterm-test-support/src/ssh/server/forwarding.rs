use super::{
    JoinSet, SECONDARY_DRAIN, SESSION_TEARDOWN, ShutdownDeadline, SocketAddr, cancelled,
    defer_future, io, server, watch,
};

pub(super) struct RemoteForwardTask {
    pub(super) cancel: watch::Sender<bool>,
    pub(super) abort: tokio::task::AbortHandle,
    pub(super) completion: tokio::sync::oneshot::Receiver<()>,
}

impl RemoteForwardTask {
    pub(super) async fn stop(mut self) {
        let _ = self.cancel.send(true);
        let deadline = ShutdownDeadline::after(SESSION_TEARDOWN);
        if deadline.timeout(&mut self.completion).await.is_err() {
            self.abort.abort();
            let _ = tokio::time::timeout(SECONDARY_DRAIN, &mut self.completion).await;
        }
    }
}

impl Drop for RemoteForwardTask {
    fn drop(&mut self) {
        let _ = self.cancel.send(true);
        self.abort.abort();
    }
}

pub(super) async fn bind_first(addresses: &[SocketAddr]) -> io::Result<tokio::net::TcpListener> {
    let mut last_error = None;
    for address in addresses {
        match tokio::net::TcpListener::bind(address).await {
            Ok(listener) => return Ok(listener),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "no loopback address to bind")
    }))
}

pub(super) async fn run_remote_forward(
    listener: tokio::net::TcpListener,
    handle: server::Handle,
    address: String,
    port: u32,
    mut cancellation: watch::Receiver<bool>,
) {
    let mut bridges = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            () = cancelled(&mut cancellation) => break,
            accepted = listener.accept() => {
                let Ok((mut socket, origin)) = accepted else { break };
                let handle = handle.clone();
                let address = address.clone();
                bridges.spawn(async move {
                    let Ok(channel) = handle.channel_open_forwarded_tcpip(
                        address,
                        port,
                        origin.ip().to_string(),
                        u32::from(origin.port()),
                    ).await else { return };
                    let mut channel = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut channel, &mut socket).await;
                });
            }
            _ = bridges.join_next(), if !bridges.is_empty() => {}
        }
    }
    drop(listener);
    bridges.abort_all();
    let deadline = ShutdownDeadline::after(SECONDARY_DRAIN);
    while !bridges.is_empty() {
        if deadline.timeout(bridges.join_next()).await.is_err() {
            break;
        }
    }
    if !bridges.is_empty() {
        let future = Box::pin(async move { while bridges.join_next().await.is_some() {} });
        let _ = defer_future(future);
    }
}
