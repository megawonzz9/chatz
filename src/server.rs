use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

pub async fn run_server() -> anyhow::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    let (tx, _rx) = broadcast::channel::<String>(10);

    println!("Server running on 0.0.0.0:8080");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("Client connected: {}", addr);

        let tx = tx.clone();
        let mut rx = tx.subscribe();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            loop {
                tokio::select! {
                    result = reader.read_line(&mut line) => {
                        if result.unwrap() == 0 {
                            println!("Client disconnected: {}", addr);
                            break;
                        }
                        let msg = format!("{}: {}", addr, line);
                        tx.send(msg).unwrap();
                        line.clear();
                    }
                    result = rx.recv() => {
                        let msg = result.unwrap();
                        if writer.write_all(msg.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });
    }
}
