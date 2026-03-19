import os
import sys
import shutil
from pathlib import Path

def clean_checkpoints(target_path):
    target = Path(target_path)
    if not target.exists():
        print(f"❌ Path not found: {target}")
        return

    checkpoint_files = [
        "checkpoint.state",
        "checkpoint.state.tmp",
        "checkpoint.axons",
        "checkpoint.axons.tmp"
    ]

    deleted_count = 0
    total_freed = 0

    print(f"🔍 Searching for checkpoints in: {target.absolute()}")

    for root, dirs, files in os.walk(target):
        # Мы ищем файлы только в папках 'baked' или их подпапках
        if 'baked' not in root:
            continue
            
        for filename in files:
            if filename in checkpoint_files:
                file_path = Path(root) / filename
                try:
                    size = file_path.stat().st_size
                    file_path.unlink()
                    deleted_count += 1
                    total_freed += size
                    print(f"  🗑️ Deleted: {file_path.relative_to(target)}")
                except Exception as e:
                    print(f"  ⚠️ Failed to delete {filename} in {root}: {e}")

    if deleted_count > 0:
        print(f"\n✅ Done! Deleted {deleted_count} files.")
        print(f"📊 Total space freed: {total_freed / (1024*1024):.2f} MB")
    else:
        print("\n✨ No checkpoint files found. System is clean.")

if __name__ == "__main__":
    if len(sys.argv) > 1:
        path = sys.argv[1]
    else:
        # По умолчанию ищем в Genesis-Models в текущей директории
        path = "Genesis-Models"
        
    clean_checkpoints(path)
