// #![feature(async_await)]
use std::{
    error::Error,
    net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs},
    time::{Duration, Instant},
};
use structopt::StructOpt;
use humanize_rs::duration::parse as duration_parse;
// use futures::executor;

mod protocol;
mod response;
mod scan;

use protocol::*;

const DEFAULT_PORT: u16 = 25565;

#[derive(StructOpt)]
enum Opts {
    #[structopt(name = "scan")]
    Scan {
        #[structopt(name = "address")]
        ip_addr: IpAddr,
        #[structopt(long = "from")]
        from: Option<u16>,
        #[structopt(long = "to")]
        to: Option<u16>,
        #[structopt(long = "timeout", parse(try_from_str = "duration_parse"))]
        timeout: Option<Duration>,
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
        Opts::Scan { ip_addr, from, to, timeout } => {
            let from = from.unwrap_or(0);
            let to = to.unwrap_or(u16::max_value());
            let timeout = timeout.unwrap_or(Duration::from_micros(1000));
            eprintln!("Address: {}, ports: {}-{}, Timeout: {:?}", ip_addr, from, to, timeout);
            let ports = scan::scan_addr_range(ip_addr, from..=to, timeout)?;
            println!("Ports: {:?}", ports);
        },
        Opts::Ping { socket_addr } => {
            let mut stream = TcpStream::connect(socket_addr)?;

            eprintln!("Handshake");
            handshake(&mut stream, socket_addr)?;
            eprintln!("Request");
            request(&mut stream)?;
            eprintln!("Response");
            let response = response(&mut stream)?;
            println!("{:?}", response);

            let ping = 1;
            eprintln!("Ping");
            crate::ping(&mut stream, ping)?;
            let timer = Instant::now();
            eprintln!("Pong");
            let pong = pong(&mut stream)?;
            println!("Time elapsed: {:#?}", timer.elapsed());
            assert_eq!(ping, pong, "Ping and Pong payloads differ");
        }
    }
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