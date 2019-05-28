#![feature(async_await)]
use futures::{executor, prelude::*};
use romio::TcpStream;
use std::{
    error::Error,
    net::{SocketAddr, ToSocketAddrs},
    ops::{Range, RangeInclusive},
    time::Instant,
};
use structopt::StructOpt;

mod protocol;
mod response;

use protocol::*;

const DEFAULT_PORT: u16 = 25565;

#[derive(StructOpt)]
enum Opts {
    #[structopt(name = "scan")]
    Scan {
        #[structopt(name = "address")]
        ip_addr: String,
        #[structopt(long = "from")]
        from: Option<u16>,
        #[structopt(long = "to")]
        to: Option<u16>,
    },
    #[structopt(name = "ping")]
    Ping {
        #[structopt(name = "address", parse(try_from_str = "minecraft_addr"))]
        /// Address with optional port
        socket_addr: SocketAddr,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts = Opts::from_args();
    match opts {
        Opts::Scan { ip_addr, from, to } => {
            let from = from.unwrap_or(49152);
            let to = to.unwrap_or(65535);
            eprintln!("Address: {}, ports: {}-{}", ip_addr, from, to);
            let ports = scan_addr_range(ip_addr, from..=to)?;
            println!("Ports: {:?}", ports);
        }
        Opts::Ping { socket_addr } => {
            let res: Result<(), Box<dyn Error>> = executor::block_on(async {
                check_with_ping(&socket_addr).await?;
                Ok(())
            });
            res?
        }
    }
    Ok(())
}

async fn check_with_ping(socket_addr: &SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect(socket_addr).await?;

    eprintln!("Handshake");
    handshake(&mut stream, socket_addr).await?;
    eprintln!("Request");
    request(&mut stream).await?;
    stream.flush().await?;
    eprintln!("Response");
    let response = response(&mut stream).await?;
    println!("{}", response);

    let ping_value = 1;
    eprintln!("Ping");
    ping(&mut stream, ping_value).await?;
    stream.flush().await?;
    let timer = Instant::now();
    eprintln!("Pong");
    let pong_value = pong(&mut stream).await?;
    println!("Time elapsed: {:#?}", timer.elapsed());
    assert_eq!(ping_value, pong_value, "Ping and Pong payloads differ");

    Ok(())
}

pub fn scan_addr_range(
    addr: String,
    range: RangeInclusive<u16>,
) -> Result<Vec<u16>, Box<dyn Error + 'static>> {
    // if from > to { return Ok(vec![]); }
    let range: Range<u16> = *range.start()..(range.end().saturating_add(1));
    let (send, recv) = futures::channel::mpsc::unbounded();
    executor::block_on(async {
        range
            .into_iter()
            .inspect(|x| eprint!("{};", x))
            .for_each(|port| {
                let socket_addr = format!("{}:{}", addr, port)
                    .to_socket_addrs()
                    .unwrap()
                    .next()
                    .ok_or("Invalid address")
                    .unwrap();
                let mut send = send.clone();
                juliex::spawn(async move {
                    if check(&socket_addr).await.is_ok() {
                        send.send(port).await.unwrap();
                    }
                    send.close_channel();
                })
            });
        Ok(recv.collect::<Vec<u16>>().await)
    })
}

async fn check(socket_addr: &SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect(socket_addr).await?;
    handshake(&mut stream, socket_addr).await?;
    request(&mut stream).await?;
    response(&mut stream).await?;
    Ok(())
}

fn minecraft_addr(s: &str) -> Result<SocketAddr, Box<dyn Error>> {
    parse_addr(s, DEFAULT_PORT)
}

fn parse_addr(s: &str, port: u16) -> Result<SocketAddr, Box<dyn Error>> {
    match s.to_socket_addrs() {
        Ok(mut addrs) => addrs.next().ok_or("no addresses".into()),
        Err(_) => match format!("{}:{}", s, port).to_socket_addrs() {
            Ok(mut addrs) => addrs.next().ok_or("no addresses".into()),
            Err(_) => unimplemented!(),
        },
    }
}
