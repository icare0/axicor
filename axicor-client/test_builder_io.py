import importlib.util
import sys
import os

# Load builder.py directly
spec = importlib.util.spec_from_file_location("builder", f"{os.path.dirname(__file__)}/axicor/builder.py")
builder_mod = importlib.util.module_from_spec(spec)
sys.modules["builder"] = builder_mod
spec.loader.exec_module(builder_mod)

BrainBuilder = builder_mod.BrainBuilder

def test_builder_io_integration():
    print("Testing BrainBuilder I/O integration...")
    builder = BrainBuilder("TestProject", "out")
    zone = builder.add_zone("V1", 64, 64, 64)
    
    # Default sync_batch_ticks = 100
    # For a 256x256 matrix:
    # payload = 65487. bytes_per_tick = 65487 // 100 = 654.
    # max_aligned_pixels = (654 // 4) * 32 = 163 * 32 = 5216.
    # 256 * 256 = 65536. 
    # 65536 // 5216 = 12.5 -> should be approximately 13 chunks.
    
    print("Adding fragmented input 'retina' (256x256)...")
    zone.add_input("retina", 256, 256)
    
    assert len(zone.inputs) > 1, f"Expected multiple chunks, got {len(zone.inputs)}"
    
    first_chunk = zone.inputs[0]
    print(f"First chunk name: {first_chunk['name']}")
    assert first_chunk["name"] == "retina_chunk_0"
    assert first_chunk["target_zone"] == "V1"
    assert first_chunk["target_type"] == "All"
    assert first_chunk["entry_z"] == "top"
    assert first_chunk["stride"] == 1
    assert "uv_rect" in first_chunk
    assert len(first_chunk["uv_rect"]) == 4
    
    print("Adding non-fragmented output 'motor' (10, 10)...")
    zone.add_output("motor", 10, 10)
    
    assert len(zone.outputs) == 1
    motor_out = zone.outputs[0]
    assert motor_out["name"] == "motor"
    assert motor_out["source_zone"] == "V1"
    assert motor_out["target_type"] == "All"
    assert motor_out["stride"] == 1
    assert "uv_rect" in motor_out
    assert "entry_z" not in motor_out
    
    print("Testing entry_z validation...")
    # Valid float string
    zone.add_input("sensor", 10, 10, entry_z="50.0")
    assert zone.inputs[-1]["entry_z"] == "50.0"
    
    # Invalid value
    try:
        zone.add_input("broken", 10, 10, entry_z="invalid")
        assert False, "Should have raised ValueError for invalid entry_z"
    except ValueError as e:
        print(f"Caught expected validation error: {e}")

    print("\n--- ALL BUILDER I/O TESTS PASSED ---")

if __name__ == "__main__":
    test_builder_io_integration()
