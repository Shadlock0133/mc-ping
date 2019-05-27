use std::{error::Error, net::{IpAddr, SocketAddr, TcpStream}, ops::{Range, RangeInclusive}, time::Duration};
use rayon::prelude::*;
// use romio::{TcpStream};
// use futures::prelude::*;
use crate::protocol::*;

pub fn scan_addr_range(addr: IpAddr, range: RangeInclusive<u16>, timeout: Duration) -> Result<Vec<u16>, Box<dyn Error + 'static>> {
    // if from > to { return Ok(vec![]); }
    let range: Range<u16> = *range.start()..(range.end() - &1);
    Ok(range
        .into_par_iter()
        // .inspect(|x| eprint!("{};", x))
        .filter(|port| {
            let socket_addr = SocketAddr::new(addr, *port);
            check(socket_addr, timeout)
                // .unwrap();true
                .is_ok()
        })
        .collect())
}

fn check(socket_addr: SocketAddr, timeout: Duration) -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect_timeout(&socket_addr, timeout)?;
    stream.set_write_timeout(None)?;
    stream.set_read_timeout(None)?;
    handshake(&mut stream, socket_addr)?;
    request(&mut stream)?;
    response(&mut stream)?;
    Ok(())
}
// pub async fn scan_addr_range(addr: IpAddr, from: u16, to: u16) -> Result<Option<u16>, Box<dyn Error + 'static>> {
//     for port in from..=to {
//         eprintln!("{}", port);
//         let socket_addr = SocketAddr::new(addr, port);
//         let stream = TcpStream::connect(&socket_addr).await?;
//         if scan_addr(stream).await? {
//             return Ok(Some(port));
//         }
//     }
//     Ok(None)
// }

// async fn scan_addr(stream: TcpStream) -> Result<bool, Box<dyn Error + 'static>> {
//     // handshake(&mut stream, socket_addr)?;
//     // request(&mut stream)?;
//     // Ok(response(&mut stream).is_ok())
//     unimplemented!()
// }