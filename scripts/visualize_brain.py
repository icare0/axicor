#!/usr/bin/env python3
"""
Axicor / Genesis - Brain Topology Visualizer
Parses brain.toml and generates a 2D cluster architecture diagram (zones and connections).
Does not require engine runtime or Baker compilation.
"""

import sys
import os
import argparse

try:
    import tomllib
except ModuleNotFoundError:
    try:
        import tomli as tomllib
    except ImportError:
        print("❌ ERROR: 'tomli' module not found. Run: pip install tomli")
        sys.exit(1)

import networkx as nx
import matplotlib.pyplot as plt

def find_brain_toml(search_path):
    """Searches for brain.toml at the specified path."""
    if os.path.isfile(search_path) and search_path.endswith(".toml"):
        return search_path
    
    potential_path = os.path.join(search_path, "brain.toml")
    if os.path.isfile(potential_path):
        return potential_path
        
    return None

def visualize_topology(toml_path, save_path=None):
    print(f"🧠 Parsing Brain DNA from: {toml_path}")
    
    with open(toml_path, "rb") as f:
        try:
            brain_data = tomllib.load(f)
        except Exception as e:
            print(f"❌ ERROR: Failed to parse TOML: {e}")
            sys.exit(1)

    zones = brain_data.get("zone", [])
    connections = brain_data.get("connection", [])

    if not zones:
        print("⚠️ Warning: No zones found in this brain.toml")
        sys.exit(0)

    # Create a directed graph
    G = nx.DiGraph()

    # Add zones
    for z in zones:
        zone_name = z.get("name", "UnknownZone")
        G.add_node(zone_name)
        print(f"  + Node: {zone_name}")

    # Add connections (Ghost Axons)
    edge_labels = {}
    for c in connections:
        src = c.get("from")
        dst = c.get("to")
        if not src or not dst:
            continue
            
        w = c.get("width", "?")
        h = c.get("height", "?")
        target_type = c.get("target_type", "All")
        matrix_name = c.get("output_matrix", "")

        # Calculate capacity (number of axons)
        capacity = f"{w}x{h}"
        if isinstance(w, int) and isinstance(h, int):
            capacity += f" ({w*h} axons)"

        # Generate arrow label
        label = f"{matrix_name}\n{capacity}"
        if target_type and target_type != "All":
            label += f"\n🎯 {target_type}"

        G.add_edge(src, dst)
        edge_labels[(src, dst)] = label
        print(f"  -> Link: {src} ===[ {w}x{h} ]==> {dst}")

    # --- Rendering (Axicor Style) ---
    plt.style.use('dark_background')
    fig, ax = plt.subplots(figsize=(14, 9))
    fig.canvas.manager.set_window_title('Axicor Brain Topology')
    ax.set_facecolor('#0d1117')
    fig.patch.set_facecolor('#0d1117')

    # Calculate node positions (spring model repels unconnected nodes)
    # Use seed for deterministic rendering
    pos = nx.spring_layout(G, k=1.5, seed=42)

    # Render nodes (Spheres)
    nx.draw_networkx_nodes(G, pos, ax=ax, node_color='#1f6feb', node_size=3500, edgecolors='#58a6ff', linewidths=2)

    # Render arrows (Ghost Axons)
    nx.draw_networkx_edges(G, pos, ax=ax, edge_color='#8b949e', arrows=True, arrowsize=25, min_source_margin=25, min_target_margin=25, connectionstyle="arc3,rad=0.1")

    # Text inside nodes
    nx.draw_networkx_labels(G, pos, ax=ax, font_size=11, font_family="sans-serif", font_color='white', font_weight='bold')

    # Text on arrows
    nx.draw_networkx_edge_labels(G, pos, edge_labels=edge_labels, font_color='#58a6ff', font_size=9, label_pos=0.4, bbox=dict(facecolor='#0d1117', edgecolor='none', alpha=0.8))

    ax.set_title(f"Brain Topology Map: {os.path.basename(os.path.dirname(toml_path))}", color='white', pad=20, fontsize=16, fontweight='bold')
    ax.axis('off')

    plt.tight_layout()

    if save_path:
        plt.savefig(save_path, dpi=200, bbox_inches='tight', facecolor='#0d1117')
        print(f"\n📸 Saved topology map to: {save_path}")
    else:
        print("\n🖥️  Opening interactive viewer...")
        plt.show()

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Visualize Axicor Brain Topology from brain.toml")
    parser.add_argument("path", nargs="?", default=".", help="Path to brain.toml or a model folder")
    parser.add_argument("--save", type=str, help="Save image to file instead of opening window")
    
    args = parser.parse_args()

    toml_path = find_brain_toml(args.path)
    
    if not toml_path:
        # Try searching in Genesis-Models
        fallback_path = os.path.join("Genesis-Models", args.path, "brain.toml")
        if os.path.exists(fallback_path):
            toml_path = fallback_path
        else:
            print(f"❌ ERROR: Could not find brain.toml in '{args.path}' or 'Genesis-Models/{args.path}'")
            sys.exit(1)

    visualize_topology(toml_path, args.save)
