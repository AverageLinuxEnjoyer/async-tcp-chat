use futures::future::join_all;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, MutexGuard};

struct Client {
    transmiter: OwnedWriteHalf,
    id: i32,
    connected: bool,
}

struct Message {
    text: String,
    id: i32,
}

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6789").await?;
    println!("Listening on port 6789.");

    let clients = Arc::new(Mutex::new(Vec::new()));

    let (mpsc_t, mut mpsc_r) = mpsc::channel::<Message>(100);

    let clients2 = Arc::clone(&clients);

    // whenever a new message is received on the mpsc channel,
    // it's broadcasted to all connected clients
    let transmiter_task = tokio::spawn(async move {
        loop {
            while let Some(resp) = mpsc_r.recv().await {
                let mut clients: MutexGuard<Vec<Client>> = clients2.lock().await;
                for client in clients.iter_mut() {
                    if !client.connected || resp.id == client.id {
                        continue;
                    }

                    if let Err(err) = client
                        .transmiter
                        .write(format!("Client[{}]: {}", client.id, resp.text).as_bytes())
                        .await
                    {
                        println!("Error in writing to a client: {}", err);
                        client.connected = false;
                    }
                }
            }
        }
    });

    // used for client ids
    let mut count = 0;

    let listening_task = tokio::spawn(async move {
        loop {
            // listens for new connections
            let (socket, addr) = listener.accept().await.unwrap();
            let (mut rx, tx) = socket.into_split();

            count += 1;

            println!("{} connected!", addr);

            {
                // pushes the client into a vec
                clients.lock().await.push(Client {
                    transmiter: tx,
                    id: count,
                    connected: true,
                });
            }

            let mpsc_t_copy = mpsc_t.clone();

            tokio::spawn(async move {
                // listens for new messages from the client
                // once received, sends it down the mpsc channel
                let id = count;

                loop {
                    let mut buf = Vec::new();
                    match rx.read_buf(&mut buf).await {
                        Ok(n) => {
                            if n == 0 {
                                return;
                            }
                        }
                        Err(err) => {
                            println!("Error in reading from a client: {}", err);
                            return;
                        }
                    }

                    if let Err(err) = mpsc_t_copy
                        .send(Message {
                            text: match String::from_utf8(buf) {
                                Ok(text) => {
                                    println!("{}: message sent", id);
                                    text
                                }
                                Err(err) => err.to_string(),
                            },
                            id,
                        })
                        .await
                    {
                        println!("Error in sending data through mpsc: {}", err);
                    }
                }
            });
        }
    });

    join_all([transmiter_task, listening_task]).await;

    Ok(())
}
