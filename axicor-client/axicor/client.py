import socket
import struct
import numpy as np
from typing import List, Dict

# C-ABI Constants (Strictly from 08_io_matrix.md)
MAX_UDP_PAYLOAD = 65507
HEADER_SIZE = 20
HEADER_FMT = "<IIIIhH"  # 20 bytes: magic, zone_hash, matrix_hash, size, reward, pad
GSIO_MAGIC = 0x4F495347
GSOO_MAGIC = 0x4F4F5347  # Genesis Standard Output

class AxicorMultiClient:
    def __init__(self, addr: tuple[str, int], matrices: List[Dict[str, int]], rx_layout: list[dict] = None, timeout: float = 2.0):
        """
        :param addr: Node address (ip, port) (External UDP In)
        :param matrices: List of dictionaries [{'zone_hash': int, 'matrix_hash': int, 'payload_size': int}]
        :param rx_layout: List of expected response chunks [{'matrix_hash': int, 'size': int}]
        :param timeout: Response timeout (Biological Amnesia)
        """
        self.addr = addr
        self.num_chunks = len(matrices)
        self.rx_layout = rx_layout or []
        self.expected_chunks = len(self.rx_layout)
        
        # 1. Single Memory Arena for the entire TX payload
        total_tx_size = sum(HEADER_SIZE + m['payload_size'] for m in matrices)
        self._tx_arena = bytearray(total_tx_size)
        
        # 2. Response Arena (RX)  Zero-Copy Assembler
        # Total size pre-calculated from rx_layout
        total_rx_size = sum(m['size'] for m in self.rx_layout)
        self._rx_arena = bytearray(total_rx_size)
        self._rx_view = memoryview(self._rx_arena)
        
        # Mapping hash -> (offset, size) for instantaneous assembly
        self._rx_map = {}
        offset = 0
        for m in self.rx_layout:
            self._rx_map[m['matrix_hash']] = (offset, m['size'])
            offset += m['size']

        # Buffer for receiving a single UDP packet
        self._udp_buf = bytearray(MAX_UDP_PAYLOAD)
        
        self.sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        # [DOD FIX] Expand OS receive buffer to 8 MB to handle burst L7 chunks
        self.sock.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 8 * 1024 * 1024)
        self.sock.settimeout(timeout)
        
        self.payload_views = []
        self._tx_packets = []
        
        # 3. Slicing TX arena (Zero-Copy)
        offset = 0
        for m in matrices:
            size = m['payload_size']
            assert size <= MAX_UDP_PAYLOAD - HEADER_SIZE, "Chunk exceeds UDP MTU!"
            
            # Pack static header once
            struct.pack_into(
                HEADER_FMT, self._tx_arena, offset,
                GSIO_MAGIC, m['zone_hash'], m['matrix_hash'], size, 0, 0
            )
            
            # View of the entire packet (Header + Payload) for the socket
            packet_view = memoryview(self._tx_arena)[offset : offset + HEADER_SIZE + size]
            self._tx_packets.append(packet_view)
            
            # View of payload only, mapped to NumPy
            payload_view = packet_view[HEADER_SIZE:]
            np_view = np.ndarray((size,), dtype=np.uint8, buffer=payload_view)
            self.payload_views.append(np_view)
            
            offset += HEADER_SIZE + size

    def step(self, reward: int = 0) -> memoryview:
        """
        Hot Loop. Zero-Copy L7 Assembler.
        """
        # 1. TX: Rapid-fire burst
        if self.num_chunks > 0:
            struct.pack_into("<h", self._tx_arena, 16, reward)

        for packet in self._tx_packets:
            self.sock.sendto(packet, self.addr)

        # 2. RX: Assembler
        if self.expected_chunks == 0:
            return self._rx_view[0:0]

        chunks_received = 0
        try:
            while chunks_received < self.expected_chunks:
                size, _ = self.sock.recvfrom_into(self._udp_buf, MAX_UDP_PAYLOAD)
                if size < HEADER_SIZE: continue

                # Parse L7 GSOO header
                magic, z_hash, m_hash, pld_size, r, p = struct.unpack_from(HEADER_FMT, self._udp_buf, 0)
                
                # Strict validation: client only accepts OUTPUTS (GSOO_MAGIC) from the node
                if magic != GSOO_MAGIC: continue

                # Direct assembly into the arena via mapping
                if m_hash in self._rx_map:
                    offset, expected_size = self._rx_map[m_hash]
                    # Copy packet payload into the correct arena location without extra allocations
                    self._rx_view[offset : offset + expected_size] = self._udp_buf[HEADER_SIZE : HEADER_SIZE + expected_size]
                    chunks_received += 1
            
            return self._rx_view
            
        except (socket.timeout, TimeoutError):
            print(f"[WARN] [GenesisClient] UDP Timeout. Received {chunks_received}/{self.expected_chunks} chunks.")
            return self._rx_view[0:0]

