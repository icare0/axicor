# Просто для быстрого удаления папки с моделями

import shutil
import os
import sys

# [DOD FIX] Terminal barrier
if input("⚠️ WARNING: This will permanently destroy all Genesis-Models. Continue? [y/N]: ").strip().lower() != 'y':
    print("Aborted.")
    sys.exit(0)
path = "Genesis-Models"
if os.path.exists(path):
    shutil.rmtree(path)
    print(f"✅ Папка {path} успешно удалена.")
else:
    print(f"ℹ️ Папка {path} не найдена.")