# 工资核算助手

桌面版本位于 [Tauri 工程](desktop/)：Rust 负责本地加密工作空间和十进制试算，HTML/CSS/JavaScript 负责界面。先制作的 [HTML 原型](prototype/index.html) 覆盖 PRD v2.0 的八类页面；视觉令牌见 [设计语言](DESIGN_SYSTEM.md)，功能差距见 [PRD 对照](PRD_GAP_ANALYSIS.md)。根目录的 Python/Tkinter 版本是早期流程原型。

## Tauri 桌面演示安装包

- macOS Apple 芯片：[SalaryAssistant-Tauri-macOS-arm64-v0.2.0.dmg](dist/SalaryAssistant-Tauri-macOS-arm64-v0.2.0.dmg)。可安装运行，尚未使用 Developer ID 签名或公证。
- Windows：构建脚本在 `.github/workflows/build-installers.yml`，需在 Windows 环境生成并验收 NSIS 安装器。

当前安装包只应用虚构数据验证界面、主体加密规则、CSV 主体校验与 Rust 公式试算。**员工/输入/结果持久化、完整税费和年终奖、版本、备份等 P0 功能尚未完成，不能用于真实薪酬核算。**

## 旧原型安装包（仅供流程验证）

- macOS（Apple 芯片）：`dist/SalaryAssistant-macOS-arm64.dmg` 是旧 Tkinter 原型的测试包，界面不符合当前设计目标。
- Windows：仓库中的 `scripts/build_windows.ps1` 和 `installer/SalaryAssistant.iss` 可在 Windows 构建 `SalaryAssistant-Windows-Setup.exe`；`.github/workflows/build-installers.yml` 配置了 macOS 和 Windows 自动构建。当前 macOS 环境不能直接构建 Windows 安装包。

当前 macOS DMG 是未使用 Developer ID 签名和公证的测试包。正式对外分发前需要 Apple Developer ID 签名、公证以及目标系统上的启动验收。Windows 安装器也需要在 Windows 上构建和测试。

## 旧原型开发运行

需要 Python 3.10+，无需第三方包：

```bash
python3 app.py
```

Windows 可用 `py app.py`。数据规则保存在系统用户目录的 `SalaryAssistant/rules.json`；导入的员工工资与考勤数据只保留在本次运行的内存中。

表格必须有 `工号`、`员工编号` 或 `employee_id` 列。其他表头直接成为公式中的字段名；例如 `基本工资 + 奖金 - 扣款`。规则按列表顺序计算，后续规则可引用先前结果。支持 `+ - * /`、比较、条件表达式及 `min`、`max`、`abs`。导出为 Excel 可打开的 UTF-8 CSV。

## 当前边界

这是用于验证流程的初版。历史表格目前只提取首个工作表第 2 行的简单同一行公式，尚不能从纯数值工资表推断规则；也尚未实现个税与社保政策计算、规则版本与生效日期、XLSX 导出和数据加密。规则导入后会提示先试算验证，**请勿直接用于正式发薪**。XLSX 数据导入目前读取首个工作表；公式单元格优先使用缓存值，若无缓存值则显示公式文本。

运行检查：

```bash
python3 -m unittest discover -s tests
```
