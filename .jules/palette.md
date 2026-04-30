## 2024-05-01 - Dashboard Overlay Accessibility
**Learning:** In dark-mode Matplotlib dashboards (like `live_dashboard.py`), dark grey text (`#444`) on a dark background (`#050505`) without a bounding box creates severe contrast and readability issues when intersecting with data lines.
**Action:** Always use high-contrast colors (e.g., `#cccccc`) and a semi-transparent dark bounding box (`bbox=dict(fc='#050505', alpha=0.8)`) for text overlaid directly on plot axes to maintain accessibility.
