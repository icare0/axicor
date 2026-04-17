import numpy as np
from genesis.decoders import PopulationDecoder

class PopulationEncoder:
    """Простой энкодер для тестирования: активирует ближайший нейрон."""
    def __init__(self, variables_count: int, neurons_per_var: int, batch_size: int):
        self.V = variables_count
        self.M = neurons_per_var
        self.B = batch_size
        self.centers = np.linspace(0.0, 1.0, self.M)

    def encode(self, values: np.ndarray) -> bytearray:
        # payload: (Batch, Var, Neurons)
        data = np.zeros((self.B, self.V, self.M), dtype=np.uint8)
        for v_idx, val in enumerate(values):
            # Находим индекс ближайшего центра
            m_idx = np.argmin(np.abs(self.centers - val))
            # Активируем этот нейрон на всех тиках батча (максимальная уверенность)
            data[:, v_idx, m_idx] = 1
        return bytearray(data.tobytes())

def test_population_decoder():
    print("Testing PopulationDecoder...")
    V, M, B = 2, 10, 5
    decoder = PopulationDecoder(V, M, B)
    encoder = PopulationEncoder(V, M, B)
    
    # 1. Тест нормального декодирования
    input_states = np.array([0.2, 0.8], dtype=np.float16)
    payload = encoder.encode(input_states)
    
    # Создаем фейковый UDP пакет с заголовком 20 байт
    full_packet = bytearray(20) + payload
    rx_view = memoryview(full_packet)
    
    decoded = decoder.decode_from(rx_view, offset=20)
    print(f"Input: {input_states} -> Decoded: {decoded}")
    
    # Погрешность из-за дискретности (10 нейронов на [0,1] -> шаг 0.11)
    assert np.allclose(decoded, input_states, atol=0.06), f"Decoding failed: {decoded}"
    
    # 2. Тест Amnesia Defense (пустой буфер)
    print("Testing Amnesia Defense (Empty View)...")
    empty_view = rx_view[0:0]
    decoded_amnesia = decoder.decode_from(empty_view, offset=20)
    print(f"Amnesia Result: {decoded_amnesia}")
    assert np.all(decoded_amnesia == 0.5), "Amnesia should return neutral 0.5"
    
    # 3. Тест частичной тишины (одна переменная без спайков)
    print("Testing Partial Silence...")
    # Создаем данные, где вторая переменная (index 1) не имеет спайков
    partial_payload = np.zeros((B, V, M), dtype=np.uint8)
    # Первая переменная: спайк на 0.3
    m_idx = np.argmin(np.abs(encoder.centers - 0.3))
    partial_payload[:, 0, m_idx] = 1
    
    full_packet_partial = bytearray(20) + partial_payload.tobytes()
    decoded_partial = decoder.decode_from(memoryview(full_packet_partial), offset=20)
    print(f"Partial Silence Result: {decoded_partial}")
    assert np.allclose(decoded_partial[0], 0.3, atol=0.06)
    assert decoded_partial[1] == 0.5, "Silent variable should default to 0.5"

    print("✅ PopulationDecoder tests passed!")

if __name__ == "__main__":
    test_population_decoder()
