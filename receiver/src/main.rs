use anyhow::Result;
use clap::Parser;
use server::{Server, ServerConfig, TcpServer, UdpServer};
use tokio::sync::broadcast;

mod cli;
mod server;
mod utils;
mod websocket;

#[tokio::main]
async fn main() -> Result<()> {
    let _logging_guard = common::logging::initialize("receiver")?;

    let args = cli::Args::parse();

    // Create an optional websocket broadcaster
    let ws_broadcaster = if args.websocket {
        log::info!("WebSocket server enabled on {}:{}", args.address, args.websocket_port);
        
        let mut ws_server = websocket::WebSocketServer::new(
            &args.address.to_string(), 
            args.websocket_port
        ).await?;
        
        let broadcaster = ws_server.get_broadcaster();
        
        // Spawn websocket server task
        tokio::spawn(async move {
            if let Err(e) = ws_server.run().await {
                log::error!("websocket server error: {e}");
            }
        });
        
        Some(broadcaster)
    } else {
        None
    };

    log::info!("start listening on {}:{}", args.address, args.port);
    let (tcp, udp) = tokio::join!(
        run_receiver::<TcpServer>(&args, ws_broadcaster.clone()), 
        run_receiver::<UdpServer>(&args, ws_broadcaster)
    );

    if let Err(error) = tcp {
        log::error!("tcp: {error}");
    }

    if let Err(error) = udp {
        log::error!("udp: {error}");
    }

    Ok(())
}

async fn run_receiver<T: Server>(
    args: &cli::Args, 
    ws_broadcaster: Option<broadcast::Sender<String>>
) -> Result<()> {
    let config = create_server_config(args, ws_broadcaster);
    let mut server = T::new(format!("{}:{}", args.address, args.port), config).await?;
    server.run().await?;

    Ok(())
}

fn create_server_config(
    args: &cli::Args, 
    ws_broadcaster: Option<broadcast::Sender<String>>
) -> ServerConfig {
    let keys = args.build_keys_map();
    let diallers = args.scenarios.as_ref().map(|s| s.diallers.clone()).unwrap_or_default();
    ServerConfig::new(&diallers, keys)
        .with_nak(args.nak)
        .with_msg_mode(args.show)
        .with_websocket(ws_broadcaster)
}
