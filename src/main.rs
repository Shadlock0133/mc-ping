use structopt::StructOpt;
use std::{
    error::Error,
    io::{Cursor, Read, Write},
    net::{AddrParseError, IpAddr, SocketAddr, TcpStream},
};

const DEFAULT_PORT: u16 = 25565;

#[derive(StructOpt)]
struct Opts {
    #[structopt(name = "address", parse(try_from_str = "minecraft_addr"))]
    ip_addr: SocketAddr,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts = Opts::from_args();
    let addr = opts.ip_addr.ip().to_string();
    let port = opts.ip_addr.port();
    let mut stream = TcpStream::connect(opts.ip_addr)?;

    // Protocol Version (-1 for unspecified)
    let mut packet = vec![];
    packet.write(&to_varint(-1))?;

    // Hostname as protocol string
    packet.write(&to_varint(addr.len() as i32))?;
    packet.write(addr.as_bytes())?;

    packet.write(&port.to_be_bytes())?;

    // Next state
    packet.write(&to_varint(1))?;

    // Send handshake
    send_packet(&mut stream, 0, &packet)?;
    // Send request
    send_packet(&mut stream, 0, &[])?;
    stream.flush()?;

    let (id, response) = read_packet(&mut stream)?;
    let json = from_proto_string(&response)?;
    
    println!("Id: {:#x}, Response: {}", id, json);

    let ping = 0i64.to_be_bytes();
    send_packet(&mut stream, 1, &ping)?;
    stream.flush()?;

    let (_, response) = read_packet(&mut stream)?;
    println!("Ping: {:?}, Pong: {:?}", ping, response);

    Ok(())
}

fn send_packet<W: Write>(mut w: W, id: i32, data: &[u8]) -> Result<(), Box<dyn Error>> {
    assert!(data.len() <= i32::max_value() as u32 as usize);
    let var_id = to_varint(id);
    let len = to_varint(data.len() as i32 + var_id.len() as i32);
    w.write(&len)?;
    w.write(&var_id)?;
    w.write(data)?;
    Ok(())
}

fn read_packet<R: Read>(mut r: R) -> Result<(i32, Vec<u8>), Box<dyn Error>> {
    let len = from_varint(&mut r)? as u32 as usize;
    let mut buffer = vec![0; len];
    r.read_exact(&mut buffer)?;
    let mut buffer = Cursor::new(buffer);
    let id = from_varint(&mut buffer)?;
    let mut data = vec![];
    buffer.read_to_end(&mut data)?;
    Ok((id, data))
}

fn to_varint(n: i32) -> Vec<u8> {
    let mut v = Vec::with_capacity(5);
    let mut n = n as u32;
    loop {
        let b = (n & 0x7f) as u8;
        n >>= 7;
        if n == 0 {
            v.push(b);
            break;
        }
        v.push(b | 0x80);
    }
    v
}

fn from_varint<R: Read>(mut r: R) -> Result<i32, Box<dyn Error>> {
    let n = &mut [0u8];
    let mut ret = 0u32;
    for i in 0..5 {
        r.read_exact(n)?;
        ret |= ((n[0] & 0x7f) as u32) << (7 * i);
        if n[0] & 0x80 == 0 {
            return Ok(ret as i32);
        }
    }
    Err("Varint too long".into())
}

fn from_proto_string(mut bytes: &[u8]) -> Result<String, Box<dyn Error>> {
    let mut bytes = &mut bytes;
    let len = from_varint(&mut bytes)? as u32 as usize;
    let string = std::str::from_utf8(&bytes[..len])?.to_string();
    Ok(string)
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn write_varints() {
        assert_eq!(to_varint(0), [0]);
        assert_eq!(to_varint(1), [1]);
        assert_eq!(to_varint(2), [2]);
        assert_eq!(to_varint(127), [127]);
        assert_eq!(to_varint(128), [128, 1]);
        assert_eq!(to_varint(2147483647), [255, 255, 255, 255, 7]);
        assert_eq!(to_varint(-1), [255, 255, 255, 255, 15]);
        assert_eq!(to_varint(-2147483648), [128, 128, 128, 128, 8]);
    }

    #[test]
    fn read_varints() -> Result<(), Box<dyn Error>> {
        assert_eq!(from_varint(&[0][..])?, 0);
        assert_eq!(from_varint(&[1][..])?, 1);
        assert_eq!(from_varint(&[2][..])?, 2);
        assert_eq!(from_varint(&[127][..])?, 127);
        assert_eq!(from_varint(&[128, 1][..])?, 128);
        assert_eq!(from_varint(&[255, 255, 255, 255, 7][..])?, 2147483647);
        assert_eq!(from_varint(&[255, 255, 255, 255, 15][..])?, -1);
        assert_eq!(from_varint(&[128, 128, 128, 128, 8][..])?, -2147483648);
        Ok(())
    }
}
