use szz_core::auth::flows::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[test]
fn test_pkce_generation() {
    let (verifier, challenge) = generate_pkce();
    assert!(!verifier.is_empty());
    assert!(!challenge.is_empty());
    assert!(!verifier.contains('+'));
    assert!(!verifier.contains('/'));
    assert!(!verifier.contains('='));
    assert!(!challenge.contains('+'));
    assert!(!challenge.contains('/'));
    assert!(!challenge.contains('='));
}

#[test]
fn test_percent_decode() {
    assert_eq!(percent_decode("hello%20world"), "hello world");
    assert_eq!(percent_decode("M.R3%2Babc%2Fxyz"), "M.R3+abc/xyz");
    assert_eq!(percent_decode("plain_text"), "plain_text");
}

#[test]
fn test_extract_code_from_http_request() {
    let request = "GET /callback?code=M.R3_MOCK_AUTH_CODE_123&state=abc123state HTTP/1.1\r\nHost: localhost:12345\r\n\r\n";
    let code = extract_code_from_http_request(request);
    assert_eq!(code, Some("M.R3_MOCK_AUTH_CODE_123".to_string()));

    let invalid_req = "GET /favicon.ico HTTP/1.1\r\nHost: localhost:12345\r\n\r\n";
    assert_eq!(extract_code_from_http_request(invalid_req), None);
}

#[tokio::test]
async fn test_loopback_listener_lifecycle() {
    let (handle, listener) = start_loopback_listener("test-client-id")
        .await
        .expect("Must start loopback listener");

    assert!(handle.port > 0);
    assert!(handle.auth_url.contains("test-client-id"));
    assert!(handle.auth_url.contains(&handle.port.to_string()));

    // Spawn server waiter
    let server_task = tokio::spawn(async move {
        await_loopback_code(listener, 5).await
    });

    // Simulate browser non-code request (e.g. favicon) first
    let mut client1 = TcpStream::connect(format!("127.0.0.1:{}", handle.port))
        .await
        .expect("Must connect to loopback port");
    let req1 = format!(
        "GET /favicon.ico HTTP/1.1\r\nHost: localhost:{}\r\nConnection: close\r\n\r\n",
        handle.port
    );
    client1.write_all(req1.as_bytes()).await.expect("Must send favicon request");
    let mut resp_buf1 = Vec::new();
    client1.read_to_end(&mut resp_buf1).await.expect("Must read 404 response");
    assert!(String::from_utf8_lossy(&resp_buf1).contains("404 Not Found"));

    // Simulate browser redirect with code on root path
    let mut client2 = TcpStream::connect(format!("127.0.0.1:{}", handle.port))
        .await
        .expect("Must connect to loopback port");
    let req2 = format!(
        "GET /?code=TEST_OAUTH_CODE_888 HTTP/1.1\r\nHost: localhost:{}\r\nConnection: close\r\n\r\n",
        handle.port
    );
    client2.write_all(req2.as_bytes()).await.expect("Must send request with code");

    let mut resp_buf2 = Vec::new();
    client2.read_to_end(&mut resp_buf2).await.expect("Must read 200 response");
    let resp_str = String::from_utf8_lossy(&resp_buf2);

    assert!(resp_str.contains("HTTP/1.1 200 OK"));
    assert!(resp_str.contains("Authentication Successful!"));

    let code_result = server_task.await.expect("Task must join").expect("Code must be extracted");
    assert_eq!(code_result, "TEST_OAUTH_CODE_888");
}
