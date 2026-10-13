use super::*;

#[test]
fn should_accept_again_at_once_after_an_error_about_one_connection() {
    for kind in [
        ErrorKind::ConnectionAborted,
        ErrorKind::ConnectionReset,
        ErrorKind::Interrupted,
    ] {
        assert_eq!(
            Acceptor::retry_delay(&io::Error::from(kind)),
            Duration::ZERO
        );
    }
}

#[test]
fn should_wait_after_running_out_of_resources() {
    let too_many_files = io::Error::from_raw_os_error(24);

    assert_eq!(Acceptor::retry_delay(&too_many_files), RESOURCE_RETRY_DELAY);
    assert_eq!(
        Acceptor::retry_delay(&io::Error::from(ErrorKind::OutOfMemory)),
        RESOURCE_RETRY_DELAY
    );
}

#[tokio::test(flavor = "current_thread")]
async fn should_accept_every_connection() {
    let acceptor = Acceptor::new(TcpListener::bind("127.0.0.1:0").await.expect("binds"));
    let address = acceptor.local_addr().expect("an address");

    for _ in 0..3 {
        let connecting = TcpStream::connect(address);
        let ((accepted, from), connected) = tokio::join!(acceptor.accept(), connecting);
        let connected = connected.expect("connects");

        assert_eq!(from, connected.local_addr().expect("an address"));
        assert_eq!(accepted.peer_addr().expect("a peer"), from);
    }
}
