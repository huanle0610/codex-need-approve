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
2. 推荐下载最新的 `CodexNeedApproveSetup-x64.msi`。
3. 双击 `.msi` 安装包，按提示安装。
4. 安装完成后会自动启动 `Codex Need Approve`，并在桌面创建快捷方式。
5. 启动后会常驻托盘，名称为 `Codex Need Approve`。

`codex-need-approve-windows-x64.zip` 是免安装版本，适合想自己管理文件位置的人。

停止程序：

- 鼠标悬停托盘图标会显示 `Codex Need Approve`。
- 右键托盘图标，选择 `About` 查看介绍。
- 右键托盘图标，选择 `Exit` 退出程序。
- 如果使用 zip 版本，也可以双击 `stop-codex-need-approve.cmd`。

卸载程序：

- 打开 Windows 设置里的“应用”，卸载 `Codex Need Approve`。

测试声音：

```powershell
.\codex-need-approve.exe --test-alert
```

## 文件说明

Release 包里通常包含：

- `CodexNeedApproveSetup-x64.msi`：推荐使用的安装包，安装到当前用户目录，创建桌面和开始菜单入口，并在安装完成后自动启动。
- `codex-need-approve-windows-x64.zip`：免安装版本，需要手动解压。
- `codex-need-approve.exe`：主程序。
- `codex-need-approve.ico`：托盘图标。
- `approval-alert.wav`：提示音。
- `start-codex-need-approve.cmd`：启动脚本。
- `stop-codex-need-approve.cmd`：停止脚本。

## 工作原理

程序会检查：

- `%LOCALAPPDATA%\Codex\Logs` 下的 Codex 日志。
- 当前可见的 Codex Desktop 窗口 UI Automation 树。

当检测到确认卡片时，会通过 Windows `PlaySoundW` 播放 `approval-alert.wav`。为了避免重复提示，同一张卡片的 UI 触发和日志触发会共享冷却时间；不同 Codex 日志/会话里的确认卡片会分别记录，即使它们的内部 request id 重复，也会再次提示。

## 注意事项

- 只支持 Windows。
- 需要 Codex Desktop 正在运行。
- 如果你使用 zip 版本并移动了 exe，请把 `.ico` 和 `.wav` 保持在 exe 同目录。
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
