extern crate shared_lib;

use shared_lib::socket_message_passing::write_message;
use std::net::TcpStream;
use std::error::Error;
use std::io::Write;
use std::time::Duration;
use std::thread;


fn main() -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:37012")?;
    for _ in 0..10{
        write_message(&mut stream, "noice")?;
        thread::sleep(Duration::from_millis(500));

    }

    Ok(())
}
