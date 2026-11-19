//! A local HTTP server for downloads.

use std::io::{Read, Write};
use std::net::TcpListener;

/// Serves one HTTP request with a file.
pub struct FileServer;

impl FileServer {
    /// Serves one request with `body` on a dedicated port and returns the file URL.
    pub fn serve(body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let address = format!(
            "http://{}/file.zip",
            listener.local_addr().expect("its address")
        );

        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("a request");
            let mut request = [0; 4096];
            let _ = stream.read(&mut request);

            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .expect("the response");
        });

        address
    }
}
