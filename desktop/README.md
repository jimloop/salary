# Tauri 桌面应用

当前入口为 `frontend/`（React + TypeScript + Vite）和 `src-tauri/`（Tauri 2 + Rust）。计算、导入和加密存储由根目录的 `crates/` 提供。

请从根目录 [README](../README.md) 完成依赖安装、开发启动、测试和平台打包；Codex 工作约定见 [AGENTS.md](../AGENTS.md)。原型和设计语言分别位于 [`prototype/`](../prototype/) 与 [DESIGN_SYSTEM.md](../DESIGN_SYSTEM.md)。

前端开发入口为 `frontend/index.html`，带虚构数据的独立界面核对入口为 `frontend/ui-review.html`。生产构建只使用前者。
