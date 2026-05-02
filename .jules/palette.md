## 2026-05-02 - Improve contrast of instructions text in dark mode dashboards
**Learning:** In Axicor's dark-mode Matplotlib dashboards, overlaying instruction text directly on plot axes using dark colors (like `#444`) makes it completely unreadable against underlying grids and backgrounds.
**Action:** When adding text overlays on matplotlib axes in dark mode, always use high-contrast text colors (e.g., `#cccccc`) and include a semi-transparent dark bounding box (e.g., `bbox=dict(fc='#050505', alpha=0.8)`) to maintain readability regardless of the data plotted underneath.
