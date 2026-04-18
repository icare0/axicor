import time
import socket
from genesis.client import GenesisMultiClient

def test_client_timeout():
    print("Testing GenesisMultiClient Timeout (Biological Amnesia)...")
    
    # Configuration: dead address and port
    dead_addr = ("127.0.0.1", 9999)
    # One dummy matrix
    matrices = [{'zone_hash': 0x1, 'matrix_hash': 0x2, 'payload_size': 128}]
    
    # Set timeout to 0.5 seconds
    client = GenesisMultiClient(dead_addr, matrices, timeout=0.5)
    
    start_time = time.perf_counter()
    
    # Attempting to execute a step. The node will not respond, triggering the timeout.
    print("Executing step (expecting timeout in 0.5s)...")
    rx = client.step(reward=0)
    
    elapsed = time.perf_counter() - start_time
    
    print(f"Elapsed time: {elapsed:.4f}s")
    print(f"Received buffer size: {len(rx)}")
    
    # 1. Verify that an empty memoryview is returned
    assert len(rx) == 0, f"Expected empty buffer on timeout, got size {len(rx)}"
    assert isinstance(rx, memoryview), "Result must be a memoryview"
    
    # 2. Verify execution time (should be around 0.5s)
    assert 0.4 <= elapsed <= 0.7, f"Timeout duration out of range: {elapsed:.4f}s"
    
    print("✅ Biological Amnesia test passed successfully.")

if __name__ == "__main__":
    test_client_timeout()
