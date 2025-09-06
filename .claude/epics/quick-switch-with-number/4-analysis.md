---
issue: 4
title: 实现数字键事件处理逻辑
analyzed: 2025-09-06T12:41:15Z
streams: 1
parallel: false
---

# Issue #4 Work Stream Analysis

## Task Overview
实现数字键事件处理逻辑 - 在交互菜单的键盘事件处理中添加数字键1-9的支持，实现按下数字键后立即执行对应配置的选择和切换。

## Dependencies
- ✅ Issue #3 已完成 - 数字标号显示已实现

## Work Streams

### Stream A: Event Handler Enhancement (Primary)
- **Agent**: rust-pro (Rust专业开发)
- **Files**: `src/cmd/interactive.rs`
- **Scope**: 修改 `handle_full_interactive_menu()` 函数中的事件处理循环
- **Can Start**: Immediately (依赖已满足)
- **Dependencies**: Issue #3 completed

## Implementation Details

### Key Event Mappings
- 数字键1-9 → 对应位置的配置项
- R/r键 → 官方配置选项 (index 0)
- E/e键 → 退出选项 (index configs.len() + 1)

### Code Location
- File: `src/cmd/interactive.rs`
- Function: `handle_full_interactive_menu()` 
- Target: Event handling match block (around lines 323-357)

### Implementation Strategy
```rust
KeyCode::Char(c) if c.is_ascii_digit() => {
    let digit = c.to_digit(10).unwrap() as usize;
    if digit >= 1 && digit <= configs.len() {
        return handle_selection_action(configs, digit);
    }
    // Invalid digit - ignore silently
}
KeyCode::Char('r') | KeyCode::Char('R') => {
    return handle_selection_action(configs, 0);
}
KeyCode::Char('e') | KeyCode::Char('E') => {
    return handle_selection_action(configs, configs.len() + 1);
}
```

## Parallel Execution Plan
Single stream execution:
1. **Stream A**: 立即开始 - 添加数字键事件处理逻辑

## Key Considerations
- 保持现有方向键、回车键、Esc键功能不变
- 数字键0无响应（因为没有对应的配置项）
- 超出范围的数字键静默忽略
- 立即执行选择，无需二次确认

## Testing Requirements
- 功能测试：数字键正确映射到配置项
- 特殊键测试：R键和E键正确工作
- 边界测试：超出范围数字键处理
- 兼容性测试：现有键盘功能保持不变

## Coordination Notes
- 与Issue #3的显示增强功能配合
- 为Issue #5的分页功能打下基础
- 不与其他任务冲突
