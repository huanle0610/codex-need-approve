# Codex Need Approve

Codex Need Approve 是一个 Windows 后台小工具：当 Codex Desktop 当前会话出现需要你确认的 approval/permission 卡片时，立即播放提示音，避免你错过确认。

它不会修改 Codex 安装包，也不会连接任何服务器。程序只做两件事：

- 监听 Codex Desktop 本地日志中的 approval 事件。
- 通过 Windows UI Automation 读取 Codex 窗口状态，发现 `Awaiting approval` / `Running ...` 卡片时播放声音。

## 适用场景

如果你遇到这种情况，它会有用：

- Codex 需要你确认命令、权限或工具调用。
- 界面里出现确认卡片，但 Windows 没有声音提示。
- 你希望卡片刚出现时就听到提示音。

## 下载和使用

1. 打开本项目的 GitHub Releases。
2. 下载最新的 `codex-need-approve-windows-x64.zip`。
3. 解压到任意目录。
4. 双击 `start-codex-need-approve.cmd` 启动。
5. 启动后会常驻托盘，名称为 `Codex Need Approve`。

停止程序：

- 右键托盘图标，选择 `Exit`。
- 或双击 `stop-codex-need-approve.cmd`。

测试声音：

```powershell
.\codex-need-approve.exe --test-alert
```

## 文件说明

Release 包里通常包含：

- `codex-need-approve.exe`：主程序。
- `codex-need-approve.ico`：托盘图标。
- `approval-alert.wav`：提示音。
- `start-codex-need-approve.cmd`：启动脚本。
- `stop-codex-need-approve.cmd`：停止脚本。

## 工作原理

程序会检查：

- `%LOCALAPPDATA%\Codex\Logs` 下的 Codex 日志。
- 当前可见的 Codex Desktop 窗口 UI Automation 树。

当检测到确认卡片时，会通过 Windows `PlaySoundW` 播放 `approval-alert.wav`。为了避免重复提示，同一张卡片的 UI 触发和日志触发会共享冷却时间。

## 注意事项

- 只支持 Windows。
- 需要 Codex Desktop 正在运行。
- 如果你移动了 exe，请把 `.ico` 和 `.wav` 保持在 exe 同目录。
- 如果 Windows 安全软件拦截未知 exe，请从 GitHub Actions / Release 自行下载或从源码构建。

## 从源码构建

```powershell
cargo build --release
```

生成文件在：

```text
target\release\codex-need-approve.exe
```

## 开发

运行测试：

```powershell
cargo test
```
