# 薪酬计算助手

离线桌面薪酬计算应用，当前版本 **0.3.12**。主程序采用 Tauri 2、React/TypeScript 与 Rust；每个主体使用独立 SQLCipher 工作空间。旧 Python 桌面版及旧 HTML 界面已移除。

## 在另一台电脑继续开发

先安装 Git、Node.js 22（版本见 `.nvmrc`）、Rust/rustup 和系统构建工具：macOS 需要 Xcode Command Line Tools；Windows 需要 Visual Studio C++ 构建工具和 WebView2。Rust 版本由 `rust-toolchain.toml` 固定。

```sh
git clone git@github.com:jimloop/salary.git
cd salary
cd desktop/frontend
npm ci
npm run build
cd ../src-tauri
cargo test --locked
cargo install tauri-cli --version 2.12.4 --locked
```

依赖安装需要联网。应用业务数据本地离线保存。首次编译可能需要较长时间。

### 双终端开发

终端一，从仓库根目录运行前端开发服务：

```sh
cd desktop/frontend
npm run dev -- --host 127.0.0.1 --port 5173 --strictPort
```

终端二，从仓库根目录运行桌面应用，连接同一个前端地址：

```sh
cd desktop/src-tauri
cargo tauri dev --config '{"build":{"devUrl":"http://127.0.0.1:5173"}}'
```

前端服务另有 `http://127.0.0.1:5173/ui-review.html` 虚构数据预览入口；增加 `?history` 可直接核对工资版本对比。该入口不访问实际主体数据库，也不作为生产构建入口。

使用 Codex 时，在克隆后的仓库目录打开项目，并阅读根目录 `AGENTS.md`。当前源码、产品行为约定、测试样本和依赖锁文件均在仓库中；具体聊天记录不随 Git 同步。

## 工程结构

| 路径 | 职责 |
| --- | --- |
| `desktop/frontend/` | React + TypeScript + Vite 界面 |
| `desktop/src-tauri/` | Tauri 命令、规则、试算版本、对比、导出与备份 |
| `crates/payroll-core/` | 十进制计算、公式校验、月薪、年终奖和候选推断 |
| `crates/payroll-import/` | CSV/XLSX 等读取、字段映射与业务校验 |
| `crates/payroll-storage/` | 按主体 SQLCipher 存储、事务和分页 |
| `tests/fixtures/mock-payroll/` | 通用虚构双主体测试样本和独立预期结果 |
| `prototype/`、`DESIGN_SYSTEM.md` | 原型与设计参考 |
| `WORKFLOW.md`、`IMPLEMENTATION_STATUS.md` | 产品流程、现状与待办 |

## 测试

从仓库根目录运行：

```sh
cargo test --locked --manifest-path crates/payroll-core/Cargo.toml
cargo test --locked --manifest-path crates/payroll-import/Cargo.toml
cargo test --locked --manifest-path crates/payroll-storage/Cargo.toml
cargo test --locked --manifest-path desktop/src-tauri/Cargo.toml
```

前端的 `npm run build` 包含 TypeScript 检查。`python3 scripts/generate_mock_payroll.py` 可重新生成独立 Python Decimal 对账样本，仅使用 Python 标准库。

## 构建安装包

先在 `desktop/frontend` 执行 `npm run build`，再在 `desktop/src-tauri` 运行对应平台命令：

```sh
cargo tauri build --bundles dmg   # macOS
cargo tauri build --bundles nsis  # Windows
```

输出位于 `desktop/src-tauri/target/release/bundle/`。macOS 使用 ad-hoc 签名，未经过 Apple 公证。Windows 安装包需在 Windows 上构建并验收。GitHub Actions 支持手动构建，也可由 `v*` 标签触发；安装包只作为 Actions artifact，不提交到源码仓库。

## 数据与同步范围

Git 仅同步源码、文档、图标、锁文件和通用虚构测试样本。`target`、`node_modules`、`dist`、安装包、导出文件及本机主体数据不进入仓库。

应用标识为 `com.salaryassistant.desktop.v3`，主体数据保存在操作系统应用数据目录下的 `workspace-v3`。另一台电脑如需继续使用现有主体业务数据，须通过应用的单主体加密备份功能另行导出和恢复；Git 不包含密码或主体数据库。绑定本机主体的手动测试副本也不进入仓库。
