---
name: quick-switch-with-number
status: closed
created: 2025-09-06T12:04:47Z
updated: 2025-09-06T15:30:00Z
completed: 2025-09-06T15:30:00Z
progress: 100%
prd: .claude/prds/quick-switch-with-number.md
github: https://github.com/Linuxdazhao/cc_auto_switch/issues/2
last_sync: 2025-09-06T15:30:00Z
---

# Epic: quick-switch-with-number

## Overview

实现数字键快速选择功能，为 `cc-switch current` 命令的交互配置选择菜单添加1-9数字键直接选择能力。该功能仅在crossterm全功能终端模式下启用，与现有方向键导航并存，超过9个配置时支持分页处理。

## Architecture Decisions

- **最小代码变更**：基于现有 `handle_full_interactive_menu()` 函数扩展，不改变整体架构
- **仅修改交互逻辑**：只在 `src/cmd/interactive.rs` 中添加数字键事件处理
- **保持向后兼容**：简单菜单模式不变，crossterm全功能模式下添加新功能
- **分页策略**：采用滑动窗口方式显示配置，每页9个配置 + 1个官方选项 + 1个退出选项
- **无新依赖**：复用现有crossterm库的KeyCode数字键支持

## Technical Approach

### Frontend Components
**交互菜单增强**
- 修改 `handle_full_interactive_menu()` 函数，添加数字键事件处理
- 在配置项显示中添加数字标号 `[1]`, `[2]` 等
- 添加分页逻辑：当配置超过9个时，显示页面信息和翻页提示

**键盘事件处理**
- 扩展现有 `match event::read()` 分支，添加 `KeyCode::Char('1')` 到 `KeyCode::Char('9')` 处理
- 实现页面导航：添加 PageUp/PageDown 或 n/p 键支持
- 保持现有方向键、回车键、Esc键功能不变

### Backend Services
**配置管理逻辑**
- 无需修改现有配置存储和读取逻辑
- 复用现有 `handle_selection_action()` 函数进行配置切换
- 保持现有的 `launch_claude_with_env()` 启动逻辑

**分页算法**
- 实现配置列表分页：`page_size = 9`, `total_pages = ceil(configs.len() / 9)`
- 计算当前页配置索引范围：`start_idx = current_page * 9`, `end_idx = min(start_idx + 9, configs.len())`
- 数字键到实际配置索引的映射：`actual_index = start_idx + (digit - 1)`

### Infrastructure
- 无新的部署要求，保持现有构建和测试流程
- 无新的依赖库，使用现有crossterm功能
- 保持跨平台兼容性（Windows、macOS、Linux）

## Implementation Strategy

### 开发阶段
1. **阶段1**：添加数字键显示和基础数字键处理（单页场景）
2. **阶段2**：实现分页逻辑和页面导航
3. **阶段3**：完善错误处理和边界情况
4. **阶段4**：添加全面测试用例

### 风险缓解
- **终端兼容性风险**：仅在crossterm全功能模式下启用，简单模式不受影响
- **用户习惯改变风险**：保持现有所有操作方式，新功能为可选增强
- **分页复杂度风险**：采用简单滑动窗口算法，避免过度复杂化

### 测试策略
- **单元测试**：分页算法、数字键映射逻辑、边界情况处理
- **集成测试**：完整的用户交互流程，包括配置切换和Claude启动
- **边界测试**：0个配置、1个配置、9个配置、10+个配置等场景
- **跨平台测试**：确保在不同操作系统下功能正常

## Tasks Created
- [ ] #3 - 添加配置项数字标号显示 (parallel: true)
- [ ] #4 - 实现数字键事件处理逻辑 (parallel: false)
- [ ] #5 - 实现分页显示和导航逻辑 (parallel: false)
- [ ] #6 - 更新操作提示和帮助信息 (parallel: true)
- [ ] #7 - 边界情况处理和错误处理 (parallel: true)
- [ ] #8 - 编写测试用例和文档更新 (parallel: true)

Total tasks: 6
Parallel tasks: 4
Sequential tasks: 2
## Dependencies

### 外部依赖
- **crossterm 库**：依赖现有版本的事件处理能力，无需升级
- **现有交互模块**：基于 `src/cmd/interactive.rs` 中的现有函数扩展

### 内部依赖
- **配置管理模块**：依赖现有的 `ConfigStorage` 和 `Configuration` 结构体
- **环境配置模块**：依赖现有的 `EnvironmentConfig` 用于Claude启动

### 技术栈依赖
- Rust 1.88.0+ （现有版本）
- crossterm 库（现有版本，无需更新）
- 现有的anyhow错误处理框架
- 现有的colored输出格式化库

## Success Criteria (Technical)

### 性能基准
- **响应延迟**：数字键按下到配置切换完成 < 200ms
- **内存开销**：新功能增加内存使用 < 1MB
- **启动时间**：不影响现有应用启动速度

### 质量门控
- **代码覆盖率**：新增代码覆盖率 > 90%
- **测试通过率**：所有现有测试100%通过，新测试100%通过
- **静态检查**：通过cargo clippy检查，无警告

### 验收标准
- **功能完整性**：数字键1-9正确映射到对应配置项
- **分页正确性**：超过9个配置时正确分页，页面导航正常
- **兼容性保证**：现有所有功能保持不变，简单菜单模式不受影响
- **错误处理**：无效按键、边界情况有适当错误提示

## Estimated Effort

### 总体时间估算
- **开发时间**：2-3个工作日
  - 数字键显示和基础处理：0.5天
  - 分页逻辑实现：1天
  - 边界处理和错误处理：0.5天
  - 测试用例编写：1天

### 资源需求
- **开发人员**：1人（单人项目）
- **测试环境**：多平台测试（通过现有CI/CD）
- **代码审查**：自我审查（单人项目）

### 关键路径
1. **数字键显示** → **数字键处理** → **分页实现** → **测试验证**
2. 最关键的是分页逻辑的正确实现，这是功能的核心复杂性所在
3. 测试用例的全面性直接影响功能的稳定性和质量

### 风险时间
- 如果crossterm事件处理出现预期外问题：+0.5天
- 如果分页逻辑需要多次调试：+0.5天
- 如果跨平台测试发现问题：+0.5天

总体来说，这是一个相对简单且风险可控的功能增强，主要工作集中在现有代码的扩展和完善测试上。
