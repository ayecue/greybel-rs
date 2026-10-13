use std::net::SocketAddr;

use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Error;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::StatusCode;

pub async fn connect(
    address: SocketAddr,
    path: &str,
    origin: Option<&str>,
) -> Result<impl Sized, Error> {
    let stream = TcpStream::connect(address)
        .await
        .expect("the tcp connection to the server succeeds");
    let mut request = format!("ws://{address}{path}")
        .into_client_request()
        .expect("the websocket url forms a client request");

    if let Some(origin) = origin {
        request.headers_mut().insert(
            "origin",
            origin.parse().expect("the origin is a valid header value"),
        );
    }

    tokio_tungstenite::client_async(request, stream)
        .await
        .map(|(socket, _)| socket)
}

pub fn refusal_status<T>(result: Result<T, Error>) -> Option<StatusCode> {
    match result {
        Err(Error::Http(response)) => Some(response.status()),
        _ => None,
    }
}
