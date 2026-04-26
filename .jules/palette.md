## 2024-05-19 - Dashboard Keyboard Shortcut Contrast
**Learning:** In dark-mode matplotlib tools, overlay text directly on the plot axes can become unreadable against underlying data or gridlines, especially when using low-contrast colors like `#444`.
**Action:** Always add a bounding box (`bbox`) with a semi-transparent dark background (`fc='#050505', alpha=0.8`) and a legible text color (`#cccccc` instead of `#444`) to guarantee text readability in data-heavy overlays.
