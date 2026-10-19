//! Shared local control transport. Remoc performs RPC/framing over OS streams.
//! Named pipe DACL denies remote clients and reserves new pipe instances for
//! the pipe owner (LocalSystem in production); RPC capabilities authorize writes.
use super::AppState;
use super::rpc::{EngineControlClient, EngineControlServerShared, EngineRpc};
use eyre::{Context, Result};
use remoc::ConnectExt;
use remoc::rtc::ServerShared;
use std::sync::Arc;

#[cfg(windows)]
pub const ENGINE_PIPE_PATH: &str = r"\\.\pipe\AceBlockerEngineControlV1";
#[cfg(target_os = "linux")]
pub const ENGINE_SOCKET_PATH: &str = "/run/ace-blocker/engine-control.sock";

#[cfg(windows)]
fn engine_pipe_security()
-> Result<interprocess::os::windows::security_descriptor::SecurityDescriptor> {
    use interprocess::os::windows::security_descriptor::SecurityDescriptor;

    // interprocess creates a new server instance on every accept(). Grant
    // FILE_CREATE_PIPE_INSTANCE (0x4) to the pipe owner, so manual user-mode
    // launches work without giving that permission to every client. The
    // production service runs as LocalSystem, which owns the pipe.
    let sddl = widestring::U16CString::from_str("D:P(A;;GA;;;SY)(A;;0x12019B;;;AU)(A;;0x4;;;OW)")?;
    SecurityDescriptor::deserialize(&sddl).wrap_err("invalid engine named pipe security")
}

#[cfg(windows)]
pub async fn serve(state: AppState) -> Result<()> {
    use interprocess::os::windows::named_pipe::{PipeListenerOptions, pipe_mode};
    let listener = PipeListenerOptions::new()
        .path(ENGINE_PIPE_PATH)
        .accept_remote(false)
        .security_descriptor(Some(engine_pipe_security()?))
        .create_tokio_duplex::<pipe_mode::Bytes>()
        .wrap_err("engine named pipe listener failed")?;
    tracing::info!("Engine control RPC listening on local named pipe");
    loop {
        let stream = listener
            .accept()
            .await
            .wrap_err("accepting engine control pipe")?;
        let state = state.clone();
        tokio::spawn(async move {
            let (rx, tx) = tokio::io::split(stream);
            let (server, client) =
                EngineControlServerShared::<_, remoc::codec::Default>::new(Arc::new(EngineRpc {
                    state,
                }));
            match remoc::Connect::io(remoc::Cfg::default(), rx, tx)
                .provide(client)
                .await
            {
                Ok(()) => {
                    if let Err(error) = server.serve().await {
                        tracing::warn!("Engine RPC session ended: {error}");
                    }
                }
                Err(error) => tracing::warn!("Engine RPC handshake failed: {error}"),
            }
        });
    }
}

#[cfg(target_os = "linux")]
pub async fn serve(state: AppState) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    use tokio::net::{UnixListener, UnixStream};
    let path = std::path::Path::new(ENGINE_SOCKET_PATH);
    std::fs::create_dir_all(path.parent().unwrap())?;
    if path.exists() {
        if UnixStream::connect(path).await.is_ok() {
            eyre::bail!("engine control socket is already in use");
        }
        std::fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o666))?;
    tracing::info!("Engine control RPC listening on Unix socket");
    loop {
        let (stream, _) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            let (rx, tx) = tokio::io::split(stream);
            let (server, client) =
                EngineControlServerShared::<_, remoc::codec::Default>::new(Arc::new(EngineRpc {
                    state,
                }));
            match remoc::Connect::io(remoc::Cfg::default(), rx, tx)
                .provide(client)
                .await
            {
                Ok(()) => {
                    if let Err(error) = server.serve().await {
                        tracing::warn!("Engine RPC session ended: {error}");
                    }
                }
                Err(error) => tracing::warn!("Engine RPC handshake failed: {error}"),
            }
        });
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
pub async fn serve(_state: AppState) -> Result<()> {
    eyre::bail!("control transport only available on Windows/Linux")
}

#[cfg(windows)]
pub async fn connect() -> Result<EngineControlClient> {
    use interprocess::os::windows::named_pipe::{pipe_mode, tokio::DuplexPipeStream};
    let stream = DuplexPipeStream::<pipe_mode::Bytes>::connect_by_path(ENGINE_PIPE_PATH)
        .await
        .wrap_err("connecting to engine named pipe")?;
    let (rx, tx) = tokio::io::split(stream);
    remoc::Connect::io(remoc::Cfg::default(), rx, tx)
        .consume()
        .await
        .wrap_err("establishing engine RPC client")
}

#[cfg(target_os = "linux")]
pub async fn connect() -> Result<EngineControlClient> {
    let stream = tokio::net::UnixStream::connect(ENGINE_SOCKET_PATH).await?;
    let (rx, tx) = tokio::io::split(stream);
    remoc::Connect::io(remoc::Cfg::default(), rx, tx)
        .consume()
        .await
        .wrap_err("establishing engine RPC client")
}

#[cfg(not(any(windows, target_os = "linux")))]
pub async fn connect() -> Result<EngineControlClient> {
    eyre::bail!("control transport only available on Windows/Linux")
}

#[cfg(all(windows, test))]
mod windows_tests {
    use super::engine_pipe_security;
    use interprocess::os::windows::named_pipe::{
        PipeListenerOptions, pipe_mode, tokio::DuplexPipeStream,
    };
    use std::time::Duration;

    #[tokio::test]
    async fn pipe_owner_can_accept_and_create_next_instance() -> eyre::Result<()> {
        let path = format!(r"\\.\pipe\AceEngineAclTest-{}", uuid::Uuid::new_v4());
        let listener = PipeListenerOptions::new()
            .path(path.as_str())
            .accept_remote(false)
            .security_descriptor(Some(engine_pipe_security()?))
            .create_tokio_duplex::<pipe_mode::Bytes>()?;

        let client = DuplexPipeStream::<pipe_mode::Bytes>::connect_by_path(path.as_str()).await?;
        // accept() creates the next instance before returning the connection.
        let server = tokio::time::timeout(Duration::from_secs(5), listener.accept()).await??;
        drop((client, server));
        Ok(())
    }
}
