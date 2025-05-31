# IME Controller (输入法状态控制器)

![License](https://img.shields.io/github/license/yourusername/ime-controller)
![GitHub release (latest by date)](https://img.shields.io/github/v/release/yourusername/ime-controller)

一个 Windows 系统托盘应用程序，用于控制中文输入法的状态。解决 Windows 中文输入法频繁自动切换到英文的烦恼。

### 主要特点：
- 🎯 监听焦点事件而非循环检测，性能开销极低
- 🔄 支持中文/英文模式自由切换
- ⚡ 后台运行，用户无感知
- 💪 稳定可靠，不影响系统性能


## 功能特点

- 🔄 自动保持输入法状态（中文/英文模式）
- 🚫 支持应用程序排除列表
- ⌨️ 全局热键快速开关（默认 Ctrl + Alt + M）
- 🚀 开机自启动选项
- 🔧 系统托盘快捷操作
- 📝 持久化配置

## 使用方法

1. 下载最新版本的发布包
2. 运行可执行文件
3. 在系统托盘中找到程序图标
4. 右键点击图标进行设置：
   - 启用/禁用自动模式
   - 选择强制中文/英文模式
   - 管理排除的应用程序
   - 设置开机自启动

## 系统要求

- Windows 10 或更高版本
- 微软拼音输入法

## 编译方法

确保已安装 Rust 开发环境，然后执行：

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

## 开源协议

本项目采用 MIT 协议开源。

## 致谢

感谢所有为本项目做出贡献的开发者！
