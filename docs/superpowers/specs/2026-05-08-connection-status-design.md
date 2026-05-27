# 连接状态展示设计方案

## 需求背景

TypeLink 是一个 Electron 桌面应用，作为 WebSocket 服务器接收手机客户端的连接和文本输入。目前托盘菜单中已有连接状态显示，但扫码窗口 (qr.html) 中缺少状态提示。需要统一在 qr.html 中展示连接状态。

## 设计决策

- **状态类型**：连接/断开两种状态（沿用现有逻辑）
- **显示位置**：qr.html 中添加状态文字提示
- **数据来源**：扩展 /api/qr 接口，与托盘菜单使用相同的检测逻辑
- **展示方式**：状态文字提示
- **更新频率**：1秒轮询

## 技术方案

### 1. API 扩展

修改 `GET /api/qr` 接口，在返回内容中增加连接状态字段：

**接口路径**：`GET /api/qr`

**响应结构**：
```json
{
  "url": "http://192.168.1.100:9527",
  "dataUrl": "data:image/png;base64,...",
  "connected": false,
  "clientCount": 0
}
```

**新增字段说明**：
- `connected`：布尔值，表示是否有客户端连接
- `clientCount`：数字，表示当前连接的客户端数量

**server.js 实现**：
- 复用现有的 `wss.clients` 对象计算连接数
- 判断 `clientCount > 0` 设置 `connected` 字段
- 逻辑与托盘菜单的检测保持一致

### 2. 前端实现

**qr.html 修改**：

1. 添加状态显示元素：
```html
<p class="status" id="status">等待连接...</p>
```

2. 添加状态更新逻辑：
```javascript
function updateStatus() {
  fetch('/api/qr')
    .then(r => r.json())
    .then(data => {
      const statusEl = document.getElementById('status');
      if (data.connected) {
        statusEl.textContent = '已连接 ✓';
        statusEl.style.color = '#4ade80';
      } else {
        statusEl.textContent = '等待连接...';
        statusEl.style.color = '#888';
      }
    });
}

setInterval(updateStatus, 1000);
updateStatus();
```

3. 添加样式：
```css
.status {
  margin-top: 12px;
  font-size: 14px;
  font-weight: 500;
  color: #888;
  transition: color 0.2s;
}
```

### 3. 状态同步机制

- **托盘菜单**：主进程定时器每秒检查连接数并更新菜单（已存在）
- **qr.html**：前端定时器每秒轮询 /api/qr 接口获取状态
- **数据一致性**：两者都查询同一个 `wss.clients` 对象

## 边界情况处理

- 服务器未启动：/api/qr 返回 500，前端显示错误信息
- 初始加载：先显示"等待连接..."，再定时更新
- 网络波动：轮询机制自然处理
- 多设备连接：支持，显示连接数量

## 实施步骤

1. 修改 `server.js` 中的 `/api/qr` 接口，增加 connected 和 clientCount 字段
2. 修改 `qr.html`，添加状态显示元素和更新逻辑
3. 测试连接和断开场景，验证状态显示正确

## 风险评估

- **低风险**：纯扩展现有接口，不影响现有功能
- **性能影响**：每1秒轮询一次，开销极小
- **兼容性**：向后兼容，旧版客户端不受影响
