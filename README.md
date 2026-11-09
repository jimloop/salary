# 薪酬计算助手

离线桌面薪酬计算应用，当前版本 **0.3.20**。主程序采用 Tauri 2、React/TypeScript 与 Rust；每个主体使用独立 SQLCipher 工作空间。旧 Python 桌面版及旧 HTML 界面已移除。

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

## 员工工资与缴费配置

在“员工与薪酬方案”选择查看月份，展开员工维护工资方案、社保、公积金及其历史记录。主体选择菜单的“主体设置”和员工页的“主体默认缴费配置”共用同一设置入口。

- 工资方案、缴费配置以所属月份匹配，首尾月份均包含在内，截止月份可留空。工资与缴费配置分别匹配，不按发放月份选取。
- 每个启用项目的基数、员工比例、单位比例分别采用“员工覆盖 → 主体默认”；未填写与明确填写 0% 不同。比例填写百分数，例如 `2.5` 表示 `2.5%`。
- 工资及缴费配置支持直接编辑，导入方案也可编辑并继续关联原批次。工资方案的月份不能重叠，相邻方案之间不能缺月。员工行编辑只保存员工自定义方案，未设置字段继续跟随主体默认；清空已有员工覆盖字段可恢复默认。历史计算快照保持不变，修改后需重新试算。
- 新试算按项目计算并舍入到分，汇总承担金额。工资或必需缴费字段缺失时阻止试算；整批试算不保存部分结果。单人页面不再手填承担金额。
- 下载新的工资方案与社保公积金配置模板。旧金额批次和旧快照继续保留，旧金额不参与新计算。导入撤销后回退默认；恢复若与新记录冲突则拒绝。
- 首次解锁升级后的主体时，仅将有效期明确、无任何已有或历史导入工资方案的旧档案月薪迁移为初始方案。月中日期或缺少有效期的资料需核对后补建；迁移只执行一次，不为撤销或已终止的方案补回档案月薪。
- 配置随主体加密备份保存。新版本快照保留采用的工资方案、逐字段来源、基数比例及逐项目金额；历史结果不会随后续配置变化改写。

`desktop/frontend/ui-review.html?employees` 提供虚构配置的界面核对入口；测试用比例不代表任何地区实际政策。

日期与月份控件使用应用内选择面板，也支持手动输入；`desktop/frontend/ui-review.html?dates` 可核对弹框内选月、跨年、闰日和 Escape 关闭行为。

方案或输入变更后，受影响所属月份的历史试算会显示“需要重新计算”（包括金额未变但基数、比例变化的情况）。工作台、员工结果、明细和版本选择均会提示；旧金额仍作为快照保留，重新试算后新版本按最新方案保存。导出版本下拉框也会标识需重算的版本。

员工行的“新增社保公积金”弹框仅查看已有项目和新增当前月份未设置项目。修改已有方案请使用该项目行的“编辑”，弹框只展示当前项目；输入框允许清空，主体默认值单独显示为继承提示。员工新增项目没有可继承默认值时，须填写完整基数和双方比例后才能计算，且不影响其他员工。

主体默认配置中，“新增配置”位于表格上方；打开及保存后默认选择未配置项目，全部配置完成后隐藏下方表单。编辑时只展示当前项目且不能修改项目类型，保存后回到新增状态。已配置项支持删除；员工自定义项删除后跟随主体默认，主体默认项删除影响依赖该项的员工。删除不改写历史计算快照，受影响版本仍提示重新计算。

通用操作按钮统一蓝底白字，删除操作使用红底白字；选中、悬停与禁用状态共用按钮样式。员工展开区的标题与同行新增按钮垂直居中对齐。

员工列表点击整行可展开或收起，编辑档案按钮不触发展开切换，也可用 Enter 或空格操作聚焦行。密码输入支持 Enter 确认，保持空密码、主体名称校验及处理中防重复提交。
