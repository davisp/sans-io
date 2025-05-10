//! A simple example of a stun client with sans-io
//!
//! This is based on the blog post here:
//!
//!     https://www.firezone.dev/blog/sans-io

use std::io::{Error, ErrorKind, Result as IoResult};

use anyhow::Result;

/// An abstract socket interface that only provides the necessary functionality
/// required by our `get_public_ip` function. I could have
trait Socket {
    async fn connect(&self, addr: &str) -> IoResult<()>;
    async fn send(&self, buf: &[u8]) -> IoResult<usize>;
    async fn recv(&self, buf: &mut [u8], timeout: u64) -> IoResult<usize>;
}

/// An example from Firezone
async fn get_public_ip(sock: impl Socket) -> Result<String> {
    sock.connect("stun.cloudflare.com:3478").await?;

    let req = utils::make_binding_request()?;
    let mut resp = vec![0u8; 1024];
    loop {
        sock.send(&req).await?;

        match sock.recv(&mut resp, 5000).await {
            Ok(num_read) => {
                resp.resize(num_read, 0);
                return utils::parse_binding_response(&resp);
            }
            Err(err) if err.kind() == ErrorKind::TimedOut => {
                continue;
            }
            err => err?,
        };
    }
}

/// A synchronous implementation using the `sans_io` "executor".
///
/// The important point here is that `async` is just sugar. We can easily
/// use a fake executor to drive the polling because if we only ever use
/// blocking calls in our implementation we'll never be pending.
mod sync {
    use std::net::UdpSocket;
    use std::time::Duration;

    use super::{IoResult, Result, Socket};

    impl Socket for UdpSocket {
        async fn connect(&self, addr: &str) -> IoResult<()> {
            self.connect(addr)
        }

        async fn send(&self, buf: &[u8]) -> IoResult<usize> {
            self.send(buf)
        }

        async fn recv(&self, buf: &mut [u8], timeout: u64) -> IoResult<usize> {
            self.set_read_timeout(Some(Duration::from_millis(timeout)))?;
            self.recv(buf)
        }
    }

    pub fn get_public_ip() -> Result<()> {
        let sock = UdpSocket::bind("0.0.0.0:0")?;

        let task = sans_io::task!(super::get_public_ip(sock));
        let ip = sans_io::run(task)?;

        println!("Sync IP: {ip}");

        Ok(())
    }
}

/// The async implementation just uses tokio
///
/// This is more what I assum most folks think of when they see async. It's
/// certainly what I originally thought of when I first started learning Rust.
mod not_sync {
    use tokio::net::UdpSocket;
    use tokio::time::{Duration, sleep};

    use super::{Error, ErrorKind, IoResult, Result, Socket};

    impl Socket for UdpSocket {
        async fn connect(&self, addr: &str) -> IoResult<()> {
            self.connect(addr).await
        }

        async fn send(&self, buf: &[u8]) -> IoResult<usize> {
            self.send(buf).await
        }

        async fn recv(&self, buf: &mut [u8], timeout: u64) -> IoResult<usize> {
            let mut timer = Box::pin(sleep(Duration::from_millis(timeout)));
            tokio::select! {
                () = &mut timer => {
                    Err(Error::new(ErrorKind::TimedOut, "recv timed out"))
                },
                res = self.recv(buf) => res,
            }
        }
    }

    #[tokio::main]
    pub async fn get_public_ip() -> Result<()> {
        let sock = UdpSocket::bind("0.0.0.0:0").await?;

        let ip = super::get_public_ip(sock).await?;
        println!("Async IP: {ip}");

        Ok(())
    }
}

fn main() -> Result<()> {
    sync::get_public_ip()?;
    not_sync::get_public_ip()?;

    Ok(())
}

// Below here is just utilities for the stun packet handling

mod utils {
    use stun::agent::TransactionId;
    use stun::message::{BINDING_REQUEST, Getter, Message};
    use stun::xoraddr::XorMappedAddress;

    pub fn make_binding_request() -> anyhow::Result<Vec<u8>> {
        let mut msg = Message::new();
        msg.build(&[
            Box::<TransactionId>::default(),
            Box::new(BINDING_REQUEST),
        ])?;

        let mut bytes = Vec::new();
        msg.write_to(&mut bytes)?;

        Ok(bytes)
    }

    pub fn parse_binding_response(data: &[u8]) -> anyhow::Result<String> {
        let mut reader = std::io::Cursor::new(data);
        let mut msg = Message::new();
        msg.read_from(&mut reader)?;
        let mut xor_addr = XorMappedAddress::default();
        if xor_addr.get_from(&msg).is_ok() {
            Ok(format!("{}", xor_addr.ip))
        } else {
            Ok("UNKNOWN".into())
        }
    }
}
