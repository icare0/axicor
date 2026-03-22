import os
import toml
import numpy as np
from typing import Dict, Any
from .builder import IoMatrixDesigner
from .encoders import PopulationEncoder, PwmEncoder
from .decoders import PwmDecoder, PopulationDecoder
from .brain import fnv1a_32

class GenesisIoContract:
    def __init__(self, zone_baked_dir: str, zone_name: str):
        self.zone_hash = fnv1a_32(zone_name.encode('utf-8'))
        
        # SDK сам знает внутреннюю структуру компилятора
        io_toml_path = os.path.join(zone_baked_dir, "BrainDNA", "io.toml")
        if not os.path.exists(io_toml_path):
            raise FileNotFoundError(f"FATAL: BrainDNA I/O manifest NOT FOUND at {io_toml_path}")
            
        with open(io_toml_path, "r", encoding="utf-8") as f:
            self.data = toml.load(f)
        
        self.inputs = {inp["name"]: inp for inp in self.data.get("input", [])}
        self.outputs = {out["name"]: out for out in self.data.get("output", [])}

    def get_client_config(self, batch_size: int) -> Dict[str, Any]:
        """Возвращает kwargs для распаковки в GenesisMultiClient."""
        matrices = []
        rx_layout = []

        # 1. TX: Входы (Битовые маски)
        for inp in self.inputs.values():
            designer = IoMatrixDesigner(inp["width"], inp["height"], is_input=True)
            matrices.append({
                "zone_hash": self.zone_hash,
                "matrix_hash": fnv1a_32(inp["name"].encode('utf-8')),
                "payload_size": designer.bytes_per_tick * batch_size
            })

        # 2. RX: Выходы (Динамическая L7-фрагментация)
        current_offset = 0
        for out in self.outputs.values():
            designer = IoMatrixDesigner(out["width"], out["height"], is_input=False)
            chunks = designer.fragment(sync_batch_ticks=batch_size)
            
            for i, chunk in enumerate(chunks):
                chunk_name = out["name"].encode('utf-8') if len(chunks) == 1 else f"{out['name']}_chunk_{i}".encode('utf-8')
                chunk_size = chunk["width"] * chunk["height"] * batch_size
                rx_layout.append({
                    "matrix_hash": fnv1a_32(chunk_name),
                    "offset": current_offset,
                    "size": chunk_size
                })
                current_offset += chunk_size

        return {"matrices": matrices, "rx_layout": rx_layout}

    def create_population_encoder(self, name: str, vars_count: int, batch_size: int, sigma: float = 0.2) -> PopulationEncoder:
        inp = self.inputs[name]
        neurons_per_var = (inp["width"] * inp["height"]) // vars_count
        return PopulationEncoder(vars_count, neurons_per_var, batch_size, sigma)

    def create_pwm_decoder(self, name: str, batch_size: int) -> PwmDecoder:
        # Прямое совпадение (нефрагментированный выход)
        if name in self.outputs:
            out = self.outputs[name]
            return PwmDecoder(out["width"] * out["height"], batch_size)
        
        # Поиск чанков: motor_out -> motor_out_chunk_0, motor_out_chunk_1, ...
        total_neurons = 0
        found = False
        for out_name, out in self.outputs.items():
            if out_name.startswith(name + "_chunk_"):
                total_neurons += out["width"] * out["height"]
                found = True
        
        if not found:
            raise KeyError(f"Output '{name}' not found in contract. Available: {list(self.outputs.keys())}")
        
        return PwmDecoder(total_neurons, batch_size)

    def create_input_facade(self, name: str, buffer: Any) -> Any:
        """
        [DOD] Генерирует Zero-Cost фасад для входного преаллоцированного буфера.
        Свойства класса жестко привязываются к индексам массива через замыкания (closures).
        """
        if name not in self.inputs:
            raise KeyError(f"Input '{name}' not found in contract.")
        
        layout = self.inputs[name].get("layout", [])

        class DynamicInputFacade:
            def __init__(self, buf):
                self.raw_buffer = buf

        # Фабрика для жесткой фиксации индекса в области видимости лямбды
        def make_prop(idx):
            return property(
                lambda self: self.raw_buffer[idx], 
                lambda self, val: self.raw_buffer.__setitem__(idx, val)
            )

        for i, var_name in enumerate(layout):
            if not var_name: 
                continue
            setattr(DynamicInputFacade, var_name, make_prop(i))

        return DynamicInputFacade(buffer)

    def create_output_facade(self, name: str, buffer: Any) -> Any:
        """
        [DOD] Генерирует Read-Only фасад для выходного буфера моторов.
        """
        # Учитываем, что выход может быть нефрагментированным или чанком 0
        target_out = None
        if name in self.outputs:
            target_out = self.outputs[name]
        elif f"{name}_chunk_0" in self.outputs:
            target_out = self.outputs[f"{name}_chunk_0"]
            
        if not target_out:
            raise KeyError(f"Output '{name}' not found in contract.")
            
        layout = target_out.get("layout", [])

        class DynamicOutputFacade:
            def __init__(self, buf):
                self.raw_buffer = buf

        # Read-only фабрика
        def make_prop(idx):
            return property(lambda self: self.raw_buffer[idx])

        for i, var_name in enumerate(layout):
            if not var_name: 
                continue
            setattr(DynamicOutputFacade, var_name, make_prop(i))

        return DynamicOutputFacade(buffer)
