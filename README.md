# IME Controller (输入法状态控制器)

![License](https://img.shields.io/github/license/rpeng666/ime-controller)
![GitHub release (latest by date)](https://img.shields.io/github/v/release/rpeng666/ime-controller)

一个 Windows 系统托盘应用程序，用于控制输入法的状态。解决 Windows 中文输入法在某些场景下频繁自动切换的苦恼，直接锁定中文或者英文。

> 如果觉得有用的话，客官赏个star呗，蟹蟹 >_<

![](./docs/image.png)

### 主要特点

- 🎯 监听焦点事件而非循环检测，性能开销极低
- 🔄 支持中文/英文模式自由切换
- ⚡ 后台运行，用户无感知
- 💪 稳定可靠，不影响系统性能

## 功能特点

- [x]  🔄 强制保持输入法语言状态（中文/英文模式）
- [x] 🚀 设置开机自启动选项
- [x] 🔧 系统托盘快捷操作
- [x] 📝 持久化配置
- [ ] 🚫 支持应用程序排除列表
- [ ] ⌨️ 全局热键快速开关（默认 Ctrl + Alt + M）

## 使用方法

1. 下载最新版本的发布包
2. 运行可执行文件
3. 在系统托盘中找到程序图标
4. 右键点击图标进行设置：
   - 启用/禁用自动模式
   - 选择强制中文/英文模式
   - 设置开机自启动

## 系统要求

- Windows 11
- 微软拼音输入法

## 编译方法

确保已安装 Rust 开发环境，然后执行（需要使用Visual Studio 2022 Developer Prompt命令行，或者有rc.exe环境变量的普通命令行也行）：

```powershell
cargo build --release
```

## 配置文件

配置文件位于可执行文件同目录下的 `config.json`，包含以下设置：

- 启用状态
- 输入法模式（中文/英文）
- 排除应用列表
- 热键设置
- 开机自启动

## 贡献指南

欢迎提交 Pull Request 或 Issue！
