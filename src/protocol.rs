use crate::response::Response;
use bytes::{buf::BufMut, BytesMut};
use futures::prelude::*;
use futures_codec::{Decoder, Encoder, FramedRead, FramedWrite};
use std::{
    convert::TryInto,
    error::Error,
    fmt::{self, Display},
    io::{self, ErrorKind, Write},
    marker::Unpin,
    net::SocketAddr,
};

/// Send handshake packet
pub async fn handshake<S: AsyncWrite + Unpin>(
    stream: S,
    ip_addr: &SocketAddr,
) -> Result<(), Box<dyn Error>> {
    let addr = ip_addr.ip().to_string();
    let port = ip_addr.port();

    if addr.len() > 255 { Err("Address too long")?; }

    // Protocol Version (-1 for unspecified)
    let mut packet = vec![];
    packet.write(&to_varint(-1))?;

    // Hostname as protocol string
    packet.write(&to_proto_string(&addr))?;

    // Port
    packet.write(&port.to_be_bytes())?;

    // Next state
    packet.write(&to_varint(1))?;

    // Send handshake
    let mut framed = FramedWrite::new(stream, PacketEncoder);
    framed
        .send(Packet {
            id: 0,
            data: packet,
        })
        .await?;
    // send_packet(&mut stream, 0, &packet)?;

    Ok(())
}

/// Send request packet
pub async fn request<W: AsyncWrite + Unpin>(stream: W) -> Result<(), Box<dyn Error>> {
    let mut framed = FramedWrite::new(stream, PacketEncoder);
    framed
        .send(Packet {
            id: 0,
            data: vec![],
        })
        .await?;
    Ok(())
}

pub async fn response<R: AsyncRead + Unpin>(stream: R) -> Result<Response, Box<dyn Error>> {
    let mut framed = FramedRead::new(stream, PacketDecoder);
    let Packet { data: response, .. } = framed.next().await.ok_or("no response")??;
    let json = from_proto_string(&response)?;
    let deser: Response = serde_json::from_str(&json).map_err(|e| {
        eprintln!("Json: {}", json);
        e
    })?;

    Ok(deser)
}

pub async fn ping<W: AsyncWrite + Unpin>(stream: W, payload: i64) -> Result<(), Box<dyn Error>> {
    let ping = payload.to_be_bytes();
    let mut framed = FramedWrite::new(stream, PacketEncoder);
    framed
        .send(Packet {
            id: 1,
            data: ping.to_vec(),
        })
        .await?;
    Ok(())
}

pub async fn pong<R: AsyncRead + Unpin>(stream: R) -> Result<i64, Box<dyn Error>> {
    let mut framed = FramedRead::new(stream, PacketDecoder);
    let Packet { data, .. } = framed.next().await.ok_or("no pong")??;
    Ok(i64::from_be_bytes(data[..].try_into()?))
}

#[derive(Debug, PartialEq)]
pub struct Packet {
    id: i32,
    data: Vec<u8>,
}

pub struct PacketDecoder;
impl Decoder for PacketDecoder {
    type Item = Packet;
    type Error = std::io::Error;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let (len, len_len) = match from_varint(&src) {
            Ok(i) => i,
            Err(VarIntError::NotEnoughData) => return Ok(None),
            Err(VarIntError::TooLong) => {
                return Err(io::Error::new(ErrorKind::InvalidData, VarIntError::TooLong))
            }
        };
        let (id, id_len) = match from_varint(&src[len_len..]) {
            Ok(i) => i,
            Err(VarIntError::NotEnoughData) => return Ok(None),
            Err(VarIntError::TooLong) => {
                return Err(io::Error::new(ErrorKind::InvalidData, VarIntError::TooLong))
            }
        };
        let data_len = len as usize - id_len as usize;
        let data_start = len_len + id_len;
        let data_end = data_start + data_len;
        match src.get(data_start..data_end) {
            Some(data) => {
                let data = data.to_owned();
                src.advance(data_end);
                Ok(Some(Packet { id, data }))
            },
            None => Ok(None),
        }
    }
}

pub struct PacketEncoder;
impl Encoder for PacketEncoder {
    type Item = Packet;
    type Error = io::Error;
    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        assert!(item.data.len() <= i32::max_value() as u32 as usize);
        let id = to_varint(item.id);
        let len = to_varint((item.data.len() + id.len()) as u32 as i32);
        let data = item.data;
        let entire_len = len.len() + id.len() + data.len();
        dst.reserve(entire_len);
        dst.put(&len);
        dst.put(&id);
        dst.put(&data);
        Ok(())
    }
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

#[derive(Debug)]
pub enum VarIntError {
    NotEnoughData,
    TooLong,
}

impl Display for VarIntError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for VarIntError {}

fn from_varint(buf: &[u8]) -> Result<(i32, usize), VarIntError> {
    let mut ret = 0u32;
    for i in 0..5 {
        let n = buf.get(i).ok_or(VarIntError::NotEnoughData)?;
        ret |= ((n & 0x7f) as u32) << (7 * i);
        if n & 0x80 == 0 {
            return Ok((ret as i32, i + 1));
        }
    }
    Err(VarIntError::TooLong)
}

fn to_proto_string(s: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(s.len() + 5);
    v.extend_from_slice(&to_varint(s.len() as i32));
    v.extend_from_slice(s.as_bytes());
    v
}

fn from_proto_string(bytes: &[u8]) -> Result<String, Box<dyn Error>> {
    let (len, offset) = from_varint(bytes)?;
    let (len, offset) = (len as u32 as usize, offset as usize);
    let string = std::str::from_utf8(&bytes[offset..][..len])?.to_string();
    Ok(string)
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
        assert_eq!(from_varint(&[0][..])?.0, 0);
        assert_eq!(from_varint(&[1][..])?.0, 1);
        assert_eq!(from_varint(&[2][..])?.0, 2);
        assert_eq!(from_varint(&[127][..])?.0, 127);
        assert_eq!(from_varint(&[128, 1][..])?.0, 128);
        assert_eq!(from_varint(&[255, 255, 255, 255, 7][..])?.0, 2147483647);
        assert_eq!(from_varint(&[255, 255, 255, 255, 15][..])?.0, -1);
        assert_eq!(from_varint(&[128, 128, 128, 128, 8][..])?.0, -2147483648);
        Ok(())
    }

    #[test]
    fn write_string() -> Result<(), Box<dyn Error>> {
        assert_eq!(to_proto_string("abc"), &[3, 97, 98, 99]);
        Ok(())
    }

    #[test]
    fn read_string() -> Result<(), Box<dyn Error>> {
        assert_eq!(from_proto_string(&[3, 97, 98, 99])?, "abc");
        Ok(())
    }

    #[test]
    fn write_test_packet() -> Result<(), Box<dyn Error>> {
        use std::collections::VecDeque;

        let mut buffer = vec![];
        let mut framed = FramedWrite::new(&mut buffer, PacketEncoder);
        let res: Result<(), Box<dyn Error>> = futures::executor::block_on(async {
            let mut packets = VecDeque::new();
            packets.push_front(Packet {
                id: 0,
                data: vec![1, 2, 3],
            });
            framed.send_all(&mut packets).await?;
            Ok(())
        });
        res?;
        assert_eq!(buffer, &[4, 0, 1, 2, 3]);
        Ok(())
    }

    #[test]
    fn handshake() -> Result<(), Box<dyn Error>> {
        use std::{net::ToSocketAddrs, str::FromStr};

        let mut buffer = vec![];
        let res: Result<(), Box<dyn Error>> = futures::executor::block_on(async {
            let addr = SocketAddr::from_str("0.0.0.0:1")?;
            super::handshake(&mut buffer, &addr).await?;
            Ok(())
        });
        res?;
        assert_eq!(
            buffer,
            &[17, 0, 255, 255, 255, 255, 15, 7, 48, 46, 48, 46, 48, 46, 48, 0, 1, 1]
        );

        let mut buffer = vec![];
        let res: Result<(), Box<dyn Error>> = futures::executor::block_on(async {
            let addr = "localhost:25565".to_socket_addrs()?.next().ok_or("Invalid address in test")?;
            super::handshake(&mut buffer, &addr).await?;
            Ok(())
        });
        res?;
        assert_eq!(
            buffer,
            &[19, 0, 255, 255, 255, 255, 15, 9, 49, 50, 55, 46, 48, 46, 48, 46, 49, 99, 221, 1]
        );
        Ok(())
    }

    #[test]
    fn request() -> Result<(), Box<dyn Error>> {
        let mut buffer = vec![];
        let res: Result<(), Box<dyn Error>> = futures::executor::block_on(async {
            super::request(&mut buffer).await?;
            Ok(())
        });
        res?;
        assert_eq!(
            buffer,
            &[1, 0]
        );
        Ok(())
    }

    #[test]
    fn read_test_packet() -> Result<(), Box<dyn Error>> {
        let raw_packet = [4, 0, 1, 2, 3];
        let mut framed = FramedRead::new(&raw_packet[..], PacketDecoder);
        let mut packet = None;
        let res: Result<(), Box<dyn Error>> = futures::executor::block_on(async {
            packet = Some(framed.next().await.ok_or("No response in test")??);
            Ok(())
        });
        res?;
        assert_eq!(
            packet,
            Some(Packet {
                id: 0,
                data: vec![1, 2, 3]
            })
        );
        Ok(())
    }
}
