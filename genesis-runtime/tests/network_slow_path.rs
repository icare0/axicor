use std::net::SocketAddr;
use tokio;
use genesis_runtime::network::geometry_client::{GeometryServer, send_geometry_request};
use genesis_runtime::network::slow_path::{GeometryRequest, GeometryResponse, NewAxon, AckNewAxon};

#[tokio::test]
async fn test_slow_path_handshake() {
    // 1. Start Geometry Server on a random available port
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = GeometryServer::bind(addr).await.expect("Failed to bind server");
    let server_addr = server.local_addr().expect("Failed to get local addr");
    let mut receiver = server.spawn();

    // 2. Mock Orchestrator handling the receiver in the background
    tokio::spawn(async move {
        while let Some((req, resp_tx)) = receiver.recv().await {
            let resp = match req {
                GeometryRequest::Handover(axon) => {
                    assert_eq!(axon.source_axon_id, 42);
                    GeometryResponse::Ack(AckNewAxon {
                        source_axon_id: axon.source_axon_id,
                        ghost_id: 100, // Allocated ghost ID
                    })
                }
                _ => GeometryResponse::Error("Unexpected request".to_string()),
            };
            let _ = resp_tx.send(resp);
        }
    });

    // 3. Send NewAxon request
    let req = GeometryRequest::Handover(NewAxon {
        source_axon_id: 42,
        entry_point: (10, 20),
        vector: (1, 0, 0),
        type_mask: 0x01,
        remaining_length: 50,
    });

    let resp = send_geometry_request(server_addr, &req).await.expect("Send failed");

    // 4. Validate Response
    match resp {
        GeometryResponse::Ack(ack) => {
            assert_eq!(ack.source_axon_id, 42);
            assert_eq!(ack.ghost_id, 100);
        }
        _ => panic!("Expected AckNewAxon"),
    }
}
