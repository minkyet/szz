use std::time::Duration;
use szz_core::ping::{
    encode_varint, ping_server, read_varint, strip_minecraft_formatting,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn test_varint_encode_and_decode_roundtrip() {
    let test_values = [0, 1, 2, 127, 128, 255, 25565, 65535, 2097151, 2147483647, -1];
    for &val in &test_values {
        let encoded = encode_varint(val);
        let mut cursor = std::io::Cursor::new(encoded);
        let decoded = read_varint(&mut cursor).await.unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
fn test_strip_minecraft_formatting() {
    assert_eq!(
        strip_minecraft_formatting("§aWelcome to §l§6SZZ §rServer!"),
        "Welcome to SZZ Server!"
    );
    assert_eq!(strip_minecraft_formatting("Plain text"), "Plain text");
}

#[tokio::test]
async fn test_slp_ping_mock_online_server() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            // Read Handshake packet length & body
            let hs_len = read_varint(&mut socket).await.unwrap();
            let mut hs_body = vec![0u8; hs_len as usize];
            socket.read_exact(&mut hs_body).await.unwrap();

            // Read Status Request packet (length=1, id=0x00)
            let req_len = read_varint(&mut socket).await.unwrap();
            assert_eq!(req_len, 1);
            let req_id = read_varint(&mut socket).await.unwrap();
            assert_eq!(req_id, 0x00);

            // Build Status Response JSON
            let response_json = r#"{
                "version": { "name": "Paper 1.20.1", "protocol": 763 },
                "players": { "max": 100, "online": 42 },
                "description": { "text": "§aOfficial §lSZZ§r Test Server" }
            }"#;
            let json_bytes = response_json.as_bytes();

            let mut packet_body = Vec::new();
            packet_body.extend_from_slice(&encode_varint(0x00)); // Packet ID 0x00
            packet_body.extend_from_slice(&encode_varint(json_bytes.len() as i32));
            packet_body.extend_from_slice(json_bytes);

            let mut framed = Vec::new();
            framed.extend_from_slice(&encode_varint(packet_body.len() as i32));
            framed.extend_from_slice(&packet_body);

            socket.write_all(&framed).await.unwrap();
            socket.flush().await.unwrap();

            // Optionally echo Ping/Pong (0x01)
            if let Ok(ping_len) = read_varint(&mut socket).await {
                if ping_len == 9 {
                    let mut ping_body = [0u8; 9];
                    if socket.read_exact(&mut ping_body).await.is_ok() {
                        let mut pong = Vec::with_capacity(10);
                        pong.push(0x09);
                        pong.extend_from_slice(&ping_body);
                        let _ = socket.write_all(&pong).await;
                    }
                }
            }
        }
    });

    let address = format!("127.0.0.1:{}", local_addr.port());
    let status = ping_server(&address, Duration::from_secs(2)).await;

    assert!(status.online);
    assert_eq!(status.players_online, Some(42));
    assert_eq!(status.players_max, Some(100));
    assert_eq!(
        status.motd.as_deref(),
        Some("Official SZZ Test Server")
    );
    assert_eq!(status.version_name.as_deref(), Some("Paper 1.20.1"));
    assert!(status.latency_ms.is_some());
}

#[tokio::test]
async fn test_slp_ping_unreachable_returns_offline() {
    // Bind and immediately drop to get an unused closed port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let address = format!("127.0.0.1:{}", port);
    let status = ping_server(&address, Duration::from_millis(500)).await;
    assert!(!status.online);
    assert!(status.players_online.is_none());
    assert!(status.latency_ms.is_none());
}
