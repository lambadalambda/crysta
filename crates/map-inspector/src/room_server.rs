//! Small loopback-only host. Simulation and data have no socket dependency.
use crate::{invalid, Result};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};

const MAX_REQUEST: usize = 2048;
const VIEWER: &str = include_str!("../web/room-slice.html");

#[derive(Debug, PartialEq, Eq)]
enum Request {
    Viewer,
    Map,
    State,
    Step(u8),
    Reset,
}

struct Head<'a> {
    method: &'a str,
    path: &'a str,
    length: usize,
    origin: Option<&'a str>,
    host: Option<&'a str>,
}
fn parse_head(headers: &str) -> Result<Head<'_>> {
    let mut lines = headers.split("\r\n");
    let line = lines
        .next()
        .ok_or_else(|| invalid("request line missing"))?;
    let parts: Vec<_> = line.split_whitespace().collect();
    let [method, path, "HTTP/1.1"] = parts.as_slice() else {
        return Err(invalid("unsupported request line").into());
    };
    let mut length = None;
    let mut supplied_origin = None;
    let mut host = None;
    for line in lines {
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| invalid("malformed header"))?;
        let value = value.trim();
        if key.eq_ignore_ascii_case("Content-Length") {
            if length.replace(value.parse::<usize>()?).is_some() {
                return Err(invalid("duplicate length").into());
            }
        } else if key.eq_ignore_ascii_case("Origin") {
            if supplied_origin.replace(value).is_some() {
                return Err(invalid("duplicate origin").into());
            }
        } else if key.eq_ignore_ascii_case("Host") {
            if host.replace(value).is_some() {
                return Err(invalid("duplicate host").into());
            }
        } else if key.eq_ignore_ascii_case("Transfer-Encoding") {
            return Err(invalid("transfer encoding unsupported").into());
        }
    }
    Ok(Head {
        method,
        path,
        length: length.unwrap_or(0),
        origin: supplied_origin,
        host,
    })
}
fn parse_request(bytes: &[u8], origin: &str) -> Result<Request> {
    let text = std::str::from_utf8(bytes)?;
    let (headers, body) = text
        .split_once("\r\n\r\n")
        .ok_or_else(|| invalid("incomplete request"))?;
    let head = parse_head(headers)?;
    if head.host != origin.strip_prefix("http://") || head.length != body.len() {
        return Err(invalid("invalid host or length").into());
    }
    if head.method == "POST" && head.origin != Some(origin) {
        return Err(invalid("cross-origin mutation rejected").into());
    }
    match (head.method, head.path, body) {
        ("GET", "/", "") => Ok(Request::Viewer),
        ("GET", "/map.bmp", "") => Ok(Request::Map),
        ("GET", "/state", "") => Ok(Request::State),
        ("POST", "/reset", "") => Ok(Request::Reset),
        ("POST", "/step", value)
            if value.len() == 1 && (b'0'..=b'4').contains(&value.as_bytes()[0]) =>
        {
            Ok(Request::Step(value.as_bytes()[0] - b'0'))
        }
        _ => Err(invalid("unsupported route or input").into()),
    }
}

fn read_request(stream: &mut TcpStream, origin: &str) -> Result<Request> {
    read_with_budget(stream, origin, Duration::from_secs(2))
}
fn read_with_budget(stream: &mut TcpStream, origin: &str, budget: Duration) -> Result<Request> {
    let deadline = Instant::now() + budget;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut bytes = Vec::new();
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::TimedOut, "request deadline exceeded")
            })?;
        stream.set_read_timeout(Some(remaining))?;
        let mut buffer = [0; 512];
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            return Err(invalid("incomplete request").into());
        }
        bytes.extend_from_slice(&buffer[..count]);
        if bytes.len() > MAX_REQUEST {
            return Err(invalid("request too large").into());
        }
        if let Some(end) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
            let head = parse_head(std::str::from_utf8(&bytes[..end])?)?;
            if head.length > 1 {
                return Err(invalid("request body too large").into());
            }
            if bytes.len() >= end + 4 + head.length {
                return parse_request(&bytes, origin);
            }
        }
    }
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    write!(stream,"HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; frame-ancestors 'none'\r\n\r\n",body.len())?;
    stream.write_all(body)
}

pub(super) fn serve(rom: &rom::Rom, port: u16) -> Result<()> {
    let mut preview = crate::room_preview::Preview::new(rom)?;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port))?;
    let origin = format!("http://{}", listener.local_addr()?);
    println!("Semantic room preview: {origin}/\nLoopback only; Ctrl-C to stop. No original CPU in the simulation loop.");
    std::io::stdout().flush()?;
    for connection in listener.incoming() {
        let mut stream = connection?;
        let result = match read_request(&mut stream, &origin) {
            Ok(Request::Viewer) => respond(
                &mut stream,
                "200 OK",
                "text/html; charset=utf-8",
                VIEWER.as_bytes(),
            ),
            Ok(Request::Map) => respond(&mut stream, "200 OK", "image/bmp", preview.bitmap()),
            Ok(request) => {
                match request {
                    Request::Step(input) => preview.step(input),
                    Request::Reset => preview.reset(),
                    _ => {}
                }
                respond(
                    &mut stream,
                    "200 OK",
                    "application/json",
                    preview.state().to_string().as_bytes(),
                )
            }
            Err(_) => respond(
                &mut stream,
                "400 Bad Request",
                "text/plain",
                b"Unsupported local request",
            ),
        };
        // A disconnected browser must not terminate the simulation host.
        if let Err(error) = result {
            eprintln!("local connection: {error}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    const ORIGIN: &str = "http://127.0.0.1:1234";
    fn request(method: &str, path: &str, headers: &str, body: &str) -> Vec<u8> {
        format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:1234\r\n{headers}\r\n{body}")
            .into_bytes()
    }
    #[test]
    fn routes_require_exact_bounded_inputs_and_same_origin_mutations() {
        assert_eq!(
            parse_request(&request("GET", "/state", "", ""), ORIGIN).unwrap(),
            Request::State
        );
        let post = |headers, body| request("POST", "/step", headers, body);
        let headers = "Content-Length: 1\r\nOrigin: http://127.0.0.1:1234\r\n";
        assert_eq!(
            parse_request(&post(headers, "1"), ORIGIN).unwrap(),
            Request::Step(1)
        );
        for body in ["", "5", "11", "-", "x"] {
            assert!(parse_request(&post(headers, body), ORIGIN).is_err());
        }
        for bad in [
            "Content-Length: 1\r\n",
            "Content-Length: 1\r\nOrigin: https://elsewhere.invalid\r\n",
            "Content-Length: 1\r\nContent-Length: 1\r\n",
            "Transfer-Encoding: chunked\r\n",
        ] {
            assert!(parse_request(&post(bad, "1"), ORIGIN).is_err());
        }
        assert!(parse_request(&request("GET", "/step", "", ""), ORIGIN).is_err());
        assert!(parse_request(&request("GET", "/../ROM", "", ""), ORIGIN).is_err());
    }
    #[test]
    fn segmented_body_uses_the_same_trimmed_length_parser() {
        for spacing in ["", " ", "  "] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
            let handle = std::thread::spawn(move || {
                let (mut server, _) = listener.accept().unwrap();
                read_with_budget(&mut server, ORIGIN, Duration::from_secs(1))
                    .map_err(|e| e.to_string())
            });
            client
                .write_all(&request(
                    "POST",
                    "/step",
                    &format!("Content-Length:{spacing}1\r\nOrigin: {ORIGIN}\r\n"),
                    "",
                ))
                .unwrap();
            std::thread::sleep(Duration::from_millis(30));
            let _ = client.write_all(b"4");
            assert_eq!(handle.join().unwrap().unwrap(), Request::Step(4));
        }
    }
    #[test]
    fn request_deadline_is_not_renewed_by_small_reads() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (sender, receiver) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let (mut server, _) = listener.accept().unwrap();
            sender
                .send(read_with_budget(&mut server, ORIGIN, Duration::from_millis(150)).is_err())
                .unwrap();
        });
        let writer = std::thread::spawn(move || {
            for _ in 0..30 {
                if client.write_all(b"G").is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        });
        assert!(receiver.recv_timeout(Duration::from_millis(700)).unwrap());
        handle.join().unwrap();
        writer.join().unwrap();
    }
}
