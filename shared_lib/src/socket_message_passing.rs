use std::error::Error;
use std::io::{Read, Write};
use std::mem::transmute;
use std::net::TcpStream;

pub fn read_message(stream: &mut TcpStream) -> Result<String, Box<dyn Error>> {
    Ok(String::from_utf8(read_bytes(stream)?)?)
}

pub fn read_bytes(stream: &mut TcpStream) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut msg_size = [0u8; 4];
    stream.read_exact(&mut msg_size)?;
    let msg_size = u32::from_le_bytes(msg_size) as usize;
    let mut result = vec![0u8; msg_size];
    stream.read_exact(&mut result[0..msg_size])?;
    Ok(result)
}

pub fn write_message(stream: &mut TcpStream, message: &str) -> Result<(), Box<dyn Error>> {
    write_bytes(stream, message.as_bytes())
}

pub fn write_bytes(stream: &mut TcpStream, bytes: &[u8]) -> Result<(), Box<dyn Error>> {
    let msg_size: [u8; 4] = unsafe { transmute((bytes.len() as u32).to_le()) };
    stream.write_all(&msg_size)?;
    stream.write_all(bytes)?;
    Ok(())
}
