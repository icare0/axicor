## 2024-04-28 - High-Contrast Overlays in Dark Mode Dashboards
**Learning:** Text overlaid directly on plot axes in dark-mode Matplotlib dashboards often becomes unreadable against underlying data grids and background lines if it uses low-contrast colors or lacks a bounding box.
**Action:** Always use high-contrast colors (e.g., `#cccccc`) and a semi-transparent dark bounding box (`bbox=dict(fc='#050505', alpha=0.8)`) for text overlaid on plot axes in Axicor's Matplotlib dashboards.
