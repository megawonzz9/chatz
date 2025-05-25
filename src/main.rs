mod client;
mod server;

use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("server") => server::run_server().await?,
        Some("client") => client::run_client().await?,
        _ => {
            eprintln!("Usage: cargo run -- <server|client>");
        }
    }
    Ok(())
}
