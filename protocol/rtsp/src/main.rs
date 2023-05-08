use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8554").unwrap();
    println!("RTSP server is listening on {}", listener.local_addr().unwrap());
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("Received connection from {}", stream.peer_addr().unwrap());
                handle_request(&mut stream);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}

fn handle_request(stream: &mut TcpStream) {
    let mut buffer = [0; 1024];
    let nbytes = stream.read(&mut buffer).unwrap();
    let request = String::from_utf8_lossy(&buffer[0..nbytes]);
    println!("Received request: {}", request);
    let lines: Vec<&str> = request.lines().collect();
    let method_url_version: Vec<&str> = lines[0].split(' ').collect();
    let method = method_url_version[0];
    let url = method_url_version[1];
    let version = method_url_version[2];
    let mut cseq = "";
    let mut session = "";
    for line in lines.iter().skip(1) {
        if line.starts_with("CSeq:") {
            cseq = line.split(':').nth(1).unwrap().trim();
        } else if line.starts_with("Session:") {
            session = line.split(':').nth(1).unwrap().trim();
        }
    }
    match method {
        "OPTIONS" => handle_options(stream, cseq),
        "DESCRIBE" => handle_describe(stream, cseq),
        "SETUP" => handle_setup(stream, cseq, session),
        "PLAY" => handle_play(stream, cseq, session),
        "PAUSE" => handle_pause(stream, cseq, session),
        "TEARDOWN" => handle_teardown(stream, cseq, session),
        _ => println!("Unsupported method: {}", method),
    }
}

fn handle_options(stream: &mut TcpStream, cseq: &str) {
    let response = format!("RTSP/1.0 200 OK\r\nCSeq: {}\r\nPublic: OPTIONS, DESCRIBE, SETUP, PLAY, PAUSE, TEARDOWN\r\n\r\n", cseq);
    stream.write(response.as_bytes()).unwrap();
}

fn handle_describe(stream: &mut TcpStream, cseq: &str) {
    let response = format!("RTSP/1.0 200 OK\r\nCSeq: {}\r\nContent-Type: application/sdp\r\n\r\n", cseq);
    stream.write(response.as_bytes()).unwrap();
}

fn handle_setup(stream: &mut TcpStream, cseq: &str, session: &str) {
    let session_id = if session.is_empty() {
        "1234567890"
    } else {
        session
    };
    let response = format!("RTSP/1.0 200 OK\r\nCSeq: {}\r\nSession: {}\r\n\r\n", cseq, session_id);
    stream.write(response.as_bytes()).unwrap();
}

fn handle_play(stream: &mut TcpStream, cseq: &str, session: &str) {
    let session_id = session;
    let response = format!("RTSP/1.0 200 OK\r\nCSeq: {}\r\nSession: {}\r\n\r\n", cseq, session_id);
    stream.write(response.as_bytes()).unwrap();
}

fn handle_pause(stream: &mut TcpStream, cseq: &str, session: &str) {
    let session_id = session;
    let response = format!("RTSP/1.0 200 OK\r\nCSeq: {}\r\nSession: {}\r\n\r\n", cseq, session_id);
    stream.write(response.as_bytes()).unwrap();
}

fn handle_teardown(stream: &mut TcpStream, cseq: &str, session: &str) {
    let session_id = session;
    let response = format!("RTSP/1.0 200 OK\r\nCSeq: {}\r\nSession: {}\r\n\r\n", cseq, session_id);
    stream.write(response.as_bytes()).unwrap();
}