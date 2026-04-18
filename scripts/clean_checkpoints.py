import os
import sys
from pathlib import Path

def clean_checkpoints(target_path):
    target = Path(target_path)
    if not target.exists():
        print(f"❌ Path not found: {target}")
        return

    # Specific checkpoint filenames
    checkpoint_names = {
        "checkpoint.state",
        "checkpoint.axons",
        "shard.state.tmp",
        "shard.axons.tmp"
    }

    deleted_count = 0
    total_freed = 0

    print(f"🔍 Deep cleaning in: {target.absolute()}")

    for root, dirs, files in os.walk(target):
        for filename in files:
            file_path = Path(root) / filename
            
            # Deletion criteria: either an exact name match or a .tmp extension
            should_delete = (filename in checkpoint_names) or filename.endswith(".tmp")
            
            if should_delete:
                try:
                    size = file_path.stat().st_size
                    file_path.unlink()
                    deleted_count += 1
                    total_freed += size
                    print(f"  🗑️ Deleted: {file_path.relative_to(target)}")
                except Exception as e:
                    print(f"  ⚠️ Failed to delete {filename} in {root}: {e}")

    if deleted_count > 0:
        print(f"\n✅ Clean-up complete! Deleted {deleted_count} files.")
        print(f"📊 Total space freed: {total_freed / (1024*1024):.2f} MB")
    else:
        print("\n✨ No temporary or checkpoint files found. System is clean.")

if __name__ == "__main__":
    # If no argument is provided, search in Genesis-Models at the project root
    if len(sys.argv) > 1:
        path = sys.argv[1]
    else:
        # Attempting to locate Genesis-Models relative to the project root
        script_dir = Path(__file__).parent
        project_root = script_dir.parent
        path = project_root / "Genesis-Models"
        
        if not path.exists():
            path = Path("Genesis-Models")
        
    clean_checkpoints(path)
