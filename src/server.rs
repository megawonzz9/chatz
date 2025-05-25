use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    sync::Arc,
};
use tokio::{
    io::{stdin, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{broadcast, watch, Mutex, RwLock},
};

type ConnectionMap = Arc<RwLock<HashMap<IpAddr, Vec<watch::Sender<()>>>>>;

pub async fn run_server() -> anyhow::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    let (tx, _rx) = broadcast::channel::<String>(100);
    let banned_ips = Arc::new(Mutex::new(HashSet::<IpAddr>::new()));
    let connections: ConnectionMap = Arc::new(RwLock::new(HashMap::new()));

    println!("Server running on 0.0.0.0:8080");

    // Komendy serwerowe
    {
        let banned_ips = banned_ips.clone();
        let tx = tx.clone();
        let connections = connections.clone();

        tokio::spawn(async move {
            let mut stdin_reader = BufReader::new(stdin());
            let mut input = String::new();

            loop {
                input.clear();
                if stdin_reader.read_line(&mut input).await.unwrap() == 0 {
                    continue;
                }

                let trimmed = input.trim();
                if trimmed.starts_with("!ban ") {
                    if let Ok(ip_to_ban) = trimmed[5..].parse::<IpAddr>() {
                        banned_ips.lock().await.insert(ip_to_ban);
                        println!(">> IP {} has been banned.", ip_to_ban);
                        let _ = tx.send(">> A user has been banned by the server.\n".to_string());

                        let mut conn_map = connections.write().await;
                        if let Some(senders) = conn_map.remove(&ip_to_ban) {
                            for sender in senders {
                                let _ = sender.send(()); // rozłączenie
                            }
                        }
                    } else {
                        println!(">> Invalid IP format.");
                    }
                } else {
                    println!(">> Unknown server command: {}", trimmed);
                }
            }
        });
    }

    loop {
        let (socket, addr) = listener.accept().await?;
        let ip = addr.ip();

        if banned_ips.lock().await.contains(&ip) {
            println!("Rejected banned IP: {}", ip);
            continue;
        }

        println!("Client connected: {}", addr);

        let tx = tx.clone();
        let mut rx = tx.subscribe();
        let (shutdown_tx, mut shutdown_rx) = watch::channel(());

        {
            let mut conn_map = connections.write().await;
            conn_map.entry(ip).or_default().push(shutdown_tx);
        }

        tokio::spawn(async move {
            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            loop {
                tokio::select! {
                    biased;

                    _ = shutdown_rx.changed() => {
                        let _ = writer.write_all(b">> You have been banned.\n").await;
                        break;
                    }

                    result = reader.read_line(&mut line) => {
                        if result.unwrap() == 0 {
                            println!("Client disconnected: {}", addr);
                            break;
                        }

                        let msg = line.trim().to_string();

                        // 🟩 Na terminalu serwera pokazujemy IP
                        println!("{}: {}", addr, msg);

                        // 🟥 Klientom wysyłamy tylko czysty tekst
                        tx.send(format!("{}\n", msg)).ok();

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

            println!("Connection closed: {}", addr);
        });
    }
}
