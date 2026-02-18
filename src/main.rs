use clap::Parser;
use std::net::IpAddr;
use tokio::net::TcpStream;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    target: String,

    #[arg(short, long, default_value_t = 1)]
    start: u16,

    #[arg(short, long, default_value_t = 1024)]
    end: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let target_ip: IpAddr = match tokio::net::lookup_host(format!("{}:80", args.target)).await {
        Ok(mut addrs) => {
            if let Some(socket_addr) = addrs.next() {
                socket_addr.ip()
            } else {
                eprintln!("Error: Could not resolve target");
                return;
            }
        },
        Err(_) => {
            eprintln!("Error: Invalid target or host");
            return;
        }
    };

    println!("Scanning target: {} ({}) [{}-{}]", args.target, target_ip, args.start, args.end);

    let (tx, mut rx) = mpsc::channel(100);

    for port in args.start..=args.end {
        let tx = tx.clone();
        
        tokio::spawn(async move {
            let timeout = tokio::time::timeout(
                Duration::from_secs(1),
                async {
                    let mut stream = TcpStream::connect((target_ip, port)).await?;
                    let mut buffer = [0; 1024];
                    let n = stream.read(&mut buffer).await.unwrap_or(0);
                    let banner = String::from_utf8_lossy(&buffer[..n]).to_string();
                    Ok::<_, std::io::Error>((port, banner))
                }
            );

            match timeout.await {
                Ok(Ok((port, banner))) => {
                    let _ = tx.send((port, banner)).await;
                },
                _ => {}
            }
        });
    }

    drop(tx);

    println!("{:<10} | {:<50}", "PORT", "SERVICE/BANNER");
    println!("{:-<65}", "-");
    
    let mut open_ports = Vec::new();
    while let Some((port, banner)) = rx.recv().await {
        let clean_banner = banner.trim().replace("\n", " ").replace("\r", "");
        let display_banner = if clean_banner.is_empty() { "Open (No banner)".to_string() } else { clean_banner };
        
        println!("{:<10} | {}", port, display_banner);
        open_ports.push(port);
    }
    
    open_ports.sort(); 
    
    if open_ports.is_empty() {
        println!("No open ports found.");
    }
    println!("{:-<65}", "-");
    println!("Scan completed.");
}
