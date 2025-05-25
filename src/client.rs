use std::io::Write;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub async fn run_client() -> anyhow::Result<()> {
    print!("Enter server IP (e.g., 192.168.1.100): ");
    std::io::stdout().flush()?;

    let mut ip = String::new();
    std::io::stdin().read_line(&mut ip)?;
    let ip = ip.trim();

    let addr = format!("{}:8080", ip);
    let stream = TcpStream::connect(addr).await?;
    let (reader, mut writer) = stream.into_split();

    let mut stdin = BufReader::new(io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut socket_reader = BufReader::new(reader);
    let mut input_line = String::new();
    let mut socket_line = String::new();

    tokio::spawn(async move {
        loop {
            socket_line.clear();
            if socket_reader.read_line(&mut socket_line).await.unwrap() == 0 {
                println!("Connection closed by server.");
                break;
            }
            print!("{}", socket_line);
            stdout.flush().await.unwrap();
        }
    });

    print!("enter nickname: ");
    std::io::stdout().flush()?;

    let mut nick = String::new();
    std::io::stdin().read_line(&mut nick)?;
    let nick = nick.trim();

    loop {
        input_line.clear();
        stdin.read_line(&mut input_line).await?;
        input_line = format!("{}: {}", nick, input_line);
        writer.write_all(input_line.as_bytes()).await?;
    }
}
