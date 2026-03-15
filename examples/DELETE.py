# Просто для быстрого удаления папки с моделями

import shutil
import os
path = "Genesis-Models"
if os.path.exists(path):
    shutil.rmtree(path)
    print(f"✅ Папка {path} успешно удалена.")
else:
    print(f"ℹ️ Папка {path} не найдена.")