## 2026-05-03 - [Matplotlib Dark Mode Overlays]
**Learning:** [In Axicor's dark-mode Matplotlib dashboards, text directly overlaid on plot axes can become unreadable when placed against underlying data grids. Using a low-contrast color like `#444` makes it almost invisible.]
**Action:** [Always use high-contrast colors (e.g., `#cccccc`) and a semi-transparent dark bounding box (`bbox=dict(fc='#050505', alpha=0.8, ec='none')`) for text overlaid on plot axes in dark-mode dashboards to ensure readability.]
