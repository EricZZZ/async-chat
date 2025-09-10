use async_chat::utils::{self, ChatResult};
use async_chat::{FromClient, FromServer};
use async_std::io::BufReader;
use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::sync::Arc;
use async_std::sync::Mutex;

pub struct Outbound(Mutex<TcpStream>);

impl Outbound {
    pub fn new(socket: TcpStream) -> Self {
        Self(Mutex::new(socket))
    }

    pub async fn send(&self, message: &FromServer) -> ChatResult<()> {
        let mut socket = self.0.lock().await;
        utils::send_as_json(&mut *socket, message).await?;
        socket.flush().await?;
        Ok(())
    }
}

use crate::group_table::GroupTable;

pub async fn serve(socket: TcpStream, groups: Arc<GroupTable>) -> ChatResult<()> {
    let outbound = Arc::new(Outbound::new(socket.clone()));

    let buffered = BufReader::new(socket);
    let mut from_client = utils::receive_as_json(buffered);

    while let Some(request_result) = from_client.next().await {
        let request = request_result?;

        let result = match request {
            FromClient::Join { group_name } => {
                let group = groups.get_or_create(group_name);
                group.join(outbound.clone());
                println!("server recived join request");
                Ok(())
            }
            FromClient::Post {
                group_name,
                message,
            } => match groups.get(&group_name) {
                Some(group) => {
                    group.post(message);
                    Ok(())
                }
                None => Err(format!("Group '{}' does not exist", group_name)),
            },
        };
        if let Err(error_message) = result {
            let report = FromServer::Error(error_message);
            outbound.send(&report).await?;
        }
    }
    Ok(())
}
