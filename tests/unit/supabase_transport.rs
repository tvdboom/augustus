use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

#[test]
fn connection_runtime_keeps_driving_tasks_between_requests() {
    let adapter = SupabaseLobbyAdapter::configured().unwrap();
    let (sender, receiver) = mpsc::channel();
    adapter.runtime.spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        sender.send(()).unwrap();
    });
    receiver.recv_timeout(Duration::from_secs(2)).expect("Idle connection tasks must keep running");
}

fn local_response(response: String) -> (SupabaseLobbyAdapter, std::thread::JoinHandle<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut adapter = SupabaseLobbyAdapter::configured().unwrap();
    adapter.client =
        reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(2)).build().unwrap();
    adapter.config = SupabaseConfig::new(
        format!("http://{}", listener.local_addr().unwrap()),
        "public-test-key",
    )
    .unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut request = Vec::new();
        let (end, length) = loop {
            let mut buffer = [0; 1024];
            let size = stream.read(&mut buffer).unwrap();
            assert!(size > 0);
            request.extend_from_slice(&buffer[..size]);
            if let Some(end) = request.windows(4).position(|s| s == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                let length: usize = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                break (end + 4, length);
            }
        };
        while request.len() < end + length {
            let mut buffer = [0; 1024];
            let size = stream.read(&mut buffer).unwrap();
            assert!(size > 0);
            request.extend_from_slice(&buffer[..size]);
        }
        stream.write_all(response.as_bytes()).unwrap();
        serde_json::from_slice(&request[end..end + length]).unwrap()
    });
    (adapter, server)
}

fn dispatch_fixture(adapter: &SupabaseLobbyAdapter) -> Response {
    let session = Session {
        access_token: "test-access-token".into(),
        refresh_token: "test-refresh-token".into(),
        expires_at: now() + 3600,
    };
    adapter
        .dispatch(Some(session), "fixture".into(), json!({"p_request_id":"unchanged"}))
        .recv_timeout(Duration::from_secs(3))
        .unwrap()
}

#[test]
fn gateway_errors_and_lost_response_bodies_are_retryable_without_leaking_details() {
    for status in [408, 429, 500, 502, 503, 504] {
        let body = "<html>private gateway diagnostics</html>";
        let (adapter, server) = local_response(format!(
            "HTTP/1.1 {status} Failed\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ));
        let response = dispatch_fixture(&adapter);
        let error = response.result.unwrap_err();
        assert_eq!(error, SERVICE_ERROR);
        assert!(retryable(&error));
        assert!(response.session.is_some());
        assert_eq!(server.join().unwrap(), json!({"p_request_id":"unchanged"}));
    }
    let (adapter, server) = local_response(
        "HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{\"revision\":".into(),
    );
    let error = dispatch_fixture(&adapter).result.unwrap_err();
    assert_eq!(error, CONNECTION_ERROR);
    assert!(retryable(&error));
    server.join().unwrap();
}

#[test]
fn sql_rejections_are_not_retried_as_network_failures() {
    let body = r#"{"message":"State conflict. Sync and retry."}"#;
    let (adapter, server) = local_response(format!(
        "HTTP/1.1 400 Bad Request\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    ));
    let error = dispatch_fixture(&adapter).result.unwrap_err();
    assert_eq!(error, "State conflict. Sync and retry.");
    assert!(!retryable(&error));
    server.join().unwrap();
}
