# Codex 配置管理

**[English](codex_EN.md) | 中文**

`cc-switch codex` 用于管理 OpenAI Codex CLI 的多个认证配置，支持 OAuth（chatgpt）和 API Key 两种认证模式。

## 快速开始

```bash
# 从现有 auth.json 导入配置
cc-switch codex add work --from-file                       # default: ~/.codex/auth.json
cc-switch codex add work --from-file ~/.codex/auth.json    # explicit path also supported

# 交互式创建配置
cc-switch codex add personal -i

# 通过 API Key 创建配置
cc-switch codex add api-test --api-key sk-xxx

# 进入交互模式
cc-switch codex

# 切换到指定配置并启动 Codex
cc-switch codex use work
```

## 命令一览

| 命令 | 作用 |
|------|------|
| `cc-switch codex` | 进入交互模式（TUI） |
| `cc-switch codex add <名称>` | 添加新配置 |
| `cc-switch codex list` | 列出所有配置 |
| `cc-switch codex use <名称>` | 切换配置并启动 Codex |
| `cc-switch codex remove <名称...>` | 删除配置 |

## 添加配置

### 从现有 auth.json 导入

```bash
# 导入已有配置（别名必须显式提供）
cc-switch codex add work --from-file
```

### 交互式创建

```bash
cc-switch codex add my-config -i

# 提示输入：
# Auth mode (chatgpt/apikey) [chatgpt]:
# ID Token:
# Access Token:
# Refresh Token:
# Account ID:
```

### API Key 模式

```bash
cc-switch codex add api-only --api-key sk-xxxxxxxx
```

### 强制覆盖

```bash
cc-switch codex add work --from-file -f
```

## 交互模式（TUI）

```bash
# 进入交互式选择界面
cc-switch codex
```

交互模式导航：

- `↑↓` / `j` `k`：上下选择配置
- `1-9`：快速选择当前页配置
- `N` / `PageDown`：下一页
- `P` / `PageUp`：上一页
- `Enter`：确认选择，切换配置并启动 Codex
- `E`：编辑当前选中的配置
- `Q`：退出不保存
- `Esc`：取消操作

每个配置会显示：
- 认证模式（apikey / chatgpt）
- 账户 ID（chatgpt 模式）
- API Key 前缀（apikey 模式）
- 上次刷新时间（如有）

## 使用配置

```bash
# 切换并启动 Codex
cc-switch codex use work

# 切换并发送提示词
cc-switch codex use work "帮我写一个 Python 脚本"

# 切换并继续最近会话
cc-switch codex use work -c

# 切换并恢复指定会话
cc-switch codex use work -r <session-id>
```

## 编辑配置

在交互模式下，选中配置后按 `E` 进入编辑模式。

可编辑字段：

| 编号 | 字段 | 说明 |
|------|------|------|
| 1 | alias_name | 别名 |
| 2 | auth_mode | 认证模式（chatgpt / apikey） |
| 3 | OPENAI_API_KEY | API 密钥 |
| 4 | id_token | ID 令牌 |
| 5 | access_token | 访问令牌 |
| 6 | refresh_token | 刷新令牌 |
| 7 | account_id | 账户 ID |
| 8 | last_refresh | 上次刷新时间 |

编辑模式操作：
- 输入编号选择要修改的字段
- 回车保持不变，输入空格清除可选字段
- `S`：保存更改
- `Q`：放弃返回

## 列出和删除配置

```bash
# 列出所有 Codex 配置（JSON 格式）
cc-switch codex list

# 纯文本格式
cc-switch codex list -p

# 删除单个配置
cc-switch codex remove work

# 删除多个配置
cc-switch codex remove work personal test
```

## 认证模式说明

### chatgpt 模式（OAuth）

使用 OpenAI 账号 OAuth 认证，包含以下令牌：

- `id_token` - 身份令牌
- `access_token` - 访问令牌
- `refresh_token` - 刷新令牌
- `account_id` - 账户 ID

适合使用 ChatGPT Plus / Team / Enterprise 订阅的用户。

### apikey 模式

使用 OpenAI API Key 认证：

- `OPENAI_API_KEY` - API 密钥

适合使用按量付费 API 的用户。

## 数据存储

Codex 配置与 Claude 配置存储在同一个文件中：`~/.claude/cc_auto_switch_setting.json`

切换配置时，工具会写入 `~/.codex/auth.json`，Codex CLI 从该文件读取认证信息。
