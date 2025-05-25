use std::{collections::HashSet, net::IpAddr, sync::Arc};
use tokio::{
    io::{stdin, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::{broadcast, Mutex},
};

pub async fn run_server() -> anyhow::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    let (tx, _rx) = broadcast::channel::<String>(10);
    let banned_ips = Arc::new(Mutex::new(HashSet::<IpAddr>::new()));

    println!("Server running on 0.0.0.0:8080");

    // Wątek serwerowy do banowania z terminala
    {
        let banned_ips = banned_ips.clone();
        let tx = tx.clone();

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
                        println!(">> IP {} has been banned by server admin.", ip_to_ban);
                        let _ = tx.send(format!(">> IP {} was banned by server.\n", ip_to_ban));
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

        let banned_ips_clone = banned_ips.clone();
        if banned_ips_clone.lock().await.contains(&ip) {
            println!("Rejected banned IP: {}", ip);
            continue;
        }

        println!("Client connected: {}", addr);

        let tx = tx.clone();
        let mut rx = tx.subscribe();
        let banned_ips = banned_ips.clone();

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

                        let msg = line.trim().to_string();

                        if msg.starts_with("!ban ") {
                            if let Ok(ip_to_ban) = msg[5..].parse::<IpAddr>() {
                                banned_ips.lock().await.insert(ip_to_ban);
                                println!(">> IP {} banned by {}", ip_to_ban, addr);
                                tx.send(format!(">> IP {} has been banned.\n", ip_to_ban)).ok();
                            } else {
                                writer.write_all(b">> Invalid IP format\n").await.ok();
                            }
                        } else {
                            tx.send(format!("{}: {}\n", addr, msg)).ok();
                        }

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
