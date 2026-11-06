# Codex 开发指引

## 当前工程

- 正在维护的是 `desktop/frontend`（React + TypeScript + Vite）与 `desktop/src-tauri`（Tauri 2 + Rust）。
- `crates/payroll-core`、`payroll-import`、`payroll-storage` 分别负责计算、文件导入和 SQLCipher 存储。
- 旧 Python 桌面版、旧 `desktop/ui` 和旧打包脚本已删除，当前仅维护 Tauri 桌面应用。
- `prototype` 与 `DESIGN_SYSTEM.md` 可作界面参考；当前应用实现及用户最新要求优先于旧规划文档。

## 开发与验证

完整环境准备和双终端开发命令见 `README.md`。Rust 版本由 `rust-toolchain.toml` 固定；Node 使用 `.nvmrc` 中的版本。使用已提交的 npm/Cargo lock 文件安装依赖。

- 前端：在 `desktop/frontend` 执行 `npm ci`、`npm run build`。
- 应用测试：`cargo test --locked --manifest-path desktop/src-tauri/Cargo.toml`。
- 核心模块测试：分别对 `crates/payroll-core`、`payroll-import`、`payroll-storage` 的 `Cargo.toml` 执行 `cargo test --locked --manifest-path ...`。
- `desktop/frontend/ui-review.html` 是带虚构数据的界面核对入口；`?history` 可核对全员/单员工工资对比。
- 发布前先构建前端资源，再用 Tauri 构建对应平台安装包；版本号同步更新两个 package JSON、Tauri 配置与应用 Cargo.toml，Cargo.lock 随构建更新。

## 已确认的产品行为

- 导入数据自动归属当前解锁主体，源文件不要求主体 ID；保留工号、期间、数值和重复记录校验。
- 每次试算保存不可覆盖的计算版本；导出通过整批试算版本选择员工结果。
- 导出文件名为“主体名称-所属年月-v版本-文档类型”；表内保留姓名、工号和实际计算数据，不重复主体、期间、内部 ID、时间和用途标识。
- 版本页支持全员逐人逐项对比、项目合计对比及单员工版本对比。未知税额/实发不能视为零或判定为完整一致。
- 员工显示优先使用姓名，并保留工号帮助区分同名员工。
- 按钮采用统一蓝色风格；数据中橙色表示差异、绿色表示一致、紫色表示数据不完整。条件控件与同行按钮统一高度并对齐。
- 地区选项使用离线完整地级名称，不要求兼容旧简称。

## 仓库边界

不提交 `target`、`node_modules`、`dist`、安装包、导出文档、本机主体数据库、密码或 SSH key。通用虚构测试资料保留在 `tests/fixtures/mock-payroll`；绑定实际本机主体的手动测试副本不进入仓库。修改前查看工作区状态，保留其他未提交工作。
