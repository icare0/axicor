use std::net::SocketAddr;
use tokio;
use genesis_runtime::network::geometry_client::{GeometryServer, send_geometry_request};
use genesis_runtime::network::slow_path::{GeometryRequest, GeometryResponse, NewAxon, AckNewAxon};
use genesis_runtime::network::router::{SpikeRouter, GhostTarget};

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

#[tokio::test]
async fn test_slow_path_sender_routing() {
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = GeometryServer::bind(addr).await.expect("Failed to bind");
    let server_addr = server.local_addr().expect("Failed to get local addr");
    let mut receiver = server.spawn();

    tokio::spawn(async move {
        while let Some((req, resp_tx)) = receiver.recv().await {
            if let GeometryRequest::Handover(axon) = req {
                let _ = resp_tx.send(GeometryResponse::Ack(AckNewAxon {
                    source_axon_id: axon.source_axon_id,
                    ghost_id: 250, // Ghost allocated on receiver
                }));
            }
        }
    });

    let mut router = SpikeRouter::new();
    let req = GeometryRequest::Handover(NewAxon {
        source_axon_id: 15,
        entry_point: (0, 0),
        vector: (1, 0, 0),
        type_mask: 0x01,
        remaining_length: 10,
    });

    // Sender performs TCP handover
    let target_node_id = 2;
    let resp = send_geometry_request(server_addr, &req).await.expect("Failed HTTP req");

    // Sender ingests Ack and updates SpikeRouter
    if let GeometryResponse::Ack(ack) = resp {
        router.add_route(ack.source_axon_id, GhostTarget {
            node_id: target_node_id,
            ghost_id: ack.ghost_id,
            tick_offset: 5,
        });
    }

    // Validate routing table propagates events natively during Day Phase
    router.route_spikes(&[15], 100);
    let out = router.flush_outgoing();
    assert_eq!(out.get(&target_node_id).unwrap()[0].receiver_ghost_id, 250);
}
