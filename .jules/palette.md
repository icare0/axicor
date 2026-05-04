## 2024-05-04 - [High Contrast Overlay Text in Matplotlib Dashboards]
**Learning:** In dark-mode Matplotlib dashboards (like `live_dashboard.py`), overlay text directly on plot axes can become unreadable if it overlaps with underlying data grids, especially if low-contrast colors (e.g., `#444`) are used.
**Action:** When adding text overlays to Matplotlib plots, use high-contrast colors (e.g., `#cccccc`) and a semi-transparent dark bounding box (`bbox=dict(fc='#050505', alpha=0.8, ec='none')`) to ensure the text remains legible regardless of the data rendered underneath.
