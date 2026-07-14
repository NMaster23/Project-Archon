use anyhow::{Result, Context};
use futures_util::{SinkExt, StreamExt};
use talos_core::{ClientToServer, ServerToClient};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::Message,
    accept_async, connect_async,
};

pub struct Connection {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

pub struct ServerConnection {
    ws: WebSocketStream<TcpStream>,
}

impl Connection {
    pub async fn send_to_server(&mut self, msg: &ClientToServer) -> Result<()> {
        let bytes = rmp_serde::to_vec(msg).context("Serialize error")?;
        self.ws.send(Message::Binary(bytes.into())).await.context("Send error")?;
        Ok(())
    }

    pub async fn recv_from_server(&mut self) -> Result<ServerToClient> {
        loop {
            match self.ws.next().await {
                Some(Ok(Message::Binary(data))) => return rmp_serde::from_slice(&data).context("Deserialize error"),
                Some(Ok(Message::Ping(data))) => { self.ws.send(Message::Pong(data)).await?; }
                Some(Ok(Message::Close(_))) => anyhow::bail!("Closed by server"),
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(e).context("Receive error"),
                None => anyhow::bail!("Stream ended"),
            }
        }
    }
}

impl ServerConnection {
    pub async fn send_to_client(&mut self, msg: &ServerToClient) -> Result<()> {
        let bytes = rmp_serde::to_vec(msg).context("Serialize error")?;
        self.ws.send(Message::Binary(bytes.into())).await.context("Send error")?;
        Ok(())
    }

    pub async fn recv_from_client(&mut self) -> Result<ClientToServer> {
        loop {
            match self.ws.next().await {
                Some(Ok(Message::Binary(data))) => return rmp_serde::from_slice(&data).context("Deserialize error"),
                Some(Ok(Message::Ping(data))) => { self.ws.send(Message::Pong(data)).await?; }
                Some(Ok(Message::Close(_))) => anyhow::bail!("Closed by client"),
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(e).context("Receive error"),
                None => anyhow::bail!("Stream ended"),
            }
        }
    }
}

pub async fn connect(url: &str) -> Result<Connection> {
    let (ws, _) = connect_async(url).await.context("Connect error")?;
    Ok(Connection { ws })
}

pub async fn listen(addr: &str) -> Result<TcpListener> {
    TcpListener::bind(addr).await.context("Bind error")
}

pub async fn accept(stream: TcpStream) -> Result<ServerConnection> {
    let ws = accept_async(stream).await.context("Handshake error")?;
    Ok(ServerConnection { ws })
}