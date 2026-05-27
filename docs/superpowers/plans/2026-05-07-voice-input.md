# Voice Input Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a desktop tray app that receives voice-to-text input from a phone and types it as keyboard input on the desktop.

**Architecture:** An Electron tray app runs an HTTP+WebSocket server locally. Phone browser opens a web page (served by desktop) via QR code scan, connects via WebSocket, sends text as JSON messages. Desktop receives text and simulates keyboard input via nut-js.

**Tech Stack:** Node.js, Electron, Express, ws, @nut-tree-fork/nut-js, qrcode, Jest

---

## File Structure

```
voice-input/                       # Project root (~/typelink)
├── .gitignore
├── desktop/
│   ├── package.json
│   ├── main.js                   # Electron main process + tray
│   ├── server.js                 # HTTP + WebSocket server
│   ├── keyboard.js               # Keyboard simulation module
│   ├── public/
│   │   ├── index.html            # Phone web page (served by Express)
│   │   └── qr.html               # QR code display page
│   └── __tests__/
│       ├── server.test.js
│       └── keyboard.test.js
└── docs/
    └── superpowers/
        ├── specs/
        │   └── 2026-05-07-voice-input-design.md
        └── plans/
            └── 2026-05-07-voice-input.md
```

---

### Task 1: Project scaffolding

**Files:**
- Create: `desktop/package.json`
- Create: `.gitignore`

- [ ] **Step 1: Write package.json**

```json
{
  "name": "voice-input",
  "version": "1.0.0",
  "description": "Use phone as voice input for desktop",
  "main": "main.js",
  "scripts": {
    "start": "electron .",
    "test": "jest"
  },
  "dependencies": {
    "@nut-tree-fork/nut-js": "^4.2.0",
    "electron": "^33.0.0",
    "express": "^4.21.0",
    "qrcode": "^1.5.4",
    "ws": "^8.18.0"
  },
  "devDependencies": {
    "jest": "^29.7.0"
  }
}
```

- [ ] **Step 2: Install dependencies**

Run: `cd desktop && npm install`

- [ ] **Step 3: Write .gitignore**

```
node_modules/
dist/
.superpowers/
```

- [ ] **Step 4: Verify structure**

Run: `ls desktop/package.json desktop/node_modules` — both should exist.

---

### Task 2: Keyboard simulation module

**Files:**
- Create: `desktop/keyboard.js`
- Create: `desktop/__tests__/keyboard.test.js`

- [ ] **Step 1: Write the failing test**

```js
// desktop/__tests__/keyboard.test.js
jest.mock('@nut-tree-fork/nut-js', () => ({
  keyboard: { type: jest.fn() }
}));

const { typeText } = require('../keyboard');
const { keyboard } = require('@nut-tree-fork/nut-js');

describe('typeText', () => {
  it('calls nut-js keyboard.type with the text', async () => {
    await typeText('你好');
    expect(keyboard.type).toHaveBeenCalledWith('你好');
  });

  it('handles empty string', async () => {
    await typeText('');
    expect(keyboard.type).toHaveBeenCalledWith('');
  });

  it('handles special characters', async () => {
    await typeText('test@email.com');
    expect(keyboard.type).toHaveBeenCalledWith('test@email.com');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd desktop && npx jest __tests__/keyboard.test.js`
Expected: FAIL — module not found

- [ ] **Step 3: Write keyboard.js implementation**

```js
// desktop/keyboard.js
const { keyboard } = require('@nut-tree-fork/nut-js');

async function typeText(text) {
  await keyboard.type(text);
}

module.exports = { typeText };
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd desktop && npx jest __tests__/keyboard.test.js`
Expected: PASS (3 tests)

- [ ] **Step 5: Commit**

```bash
git add desktop/keyboard.js desktop/__tests__/keyboard.test.js
git commit -m "feat: add keyboard simulation module using nut-js"
```

---

### Task 3: HTTP + WebSocket server

**Files:**
- Create: `desktop/server.js`
- Create: `desktop/__tests__/server.test.js`

- [ ] **Step 1: Write the failing test**

```js
// desktop/__tests__/server.test.js
const http = require('http');
const { WebSocket } = require('ws');

// We'll test against a running server instance
// Mock keyboard to avoid actual typing during tests
jest.mock('../keyboard', () => ({ typeText: jest.fn() }));

const { startServer, getLocalIP } = require('../server');

let serverInfo;

beforeAll((done) => {
  serverInfo = startServer(19527); // use non-default port for tests
  serverInfo.server.on('listening', done);
});

afterAll((done) => {
  serverInfo.server.close(done);
});

describe('HTTP server', () => {
  it('serves static files from public/', (done) => {
    http.get(`http://localhost:${serverInfo.port}/`, (res) => {
      expect(res.statusCode).toBe(200);
      let data = '';
      res.on('data', (chunk) => { data += chunk; });
      res.on('end', () => {
        expect(data).toContain('<!DOCTYPE html>');
        done();
      });
    });
  });

  it('returns 404 for non-existent files', (done) => {
    http.get(`http://localhost:${serverInfo.port}/nonexistent`, (res) => {
      expect(res.statusCode).toBe(404);
      done();
    });
  });
});

describe('WebSocket server', () => {
  it('accepts text messages and responds with ack', (done) => {
    const ws = new WebSocket(`ws://localhost:${serverInfo.port}`);

    ws.on('open', () => {
      ws.send(JSON.stringify({ type: 'text', payload: 'hello' }));
    });

    ws.on('message', (data) => {
      const msg = JSON.parse(data.toString());
      expect(msg).toEqual({ type: 'ack', payload: 'ok' });
      ws.close();
      done();
    });
  });

  it('responds to ping with pong', (done) => {
    const ws = new WebSocket(`ws://localhost:${serverInfo.port}`);

    ws.on('open', () => {
      ws.send(JSON.stringify({ type: 'ping' }));
    });

    ws.on('message', (data) => {
      const msg = JSON.parse(data.toString());
      expect(msg).toEqual({ type: 'pong' });
      ws.close();
      done();
    });
  });

  it('ignores invalid JSON', (done) => {
    const ws = new WebSocket(`ws://localhost:${serverInfo.port}`);

    ws.on('open', () => {
      ws.send('not json');
      // If no crash and no response, test passes
      setTimeout(() => {
        ws.close();
        done();
      }, 200);
    });
  });
});

describe('getLocalIP', () => {
  it('returns a non-empty string', () => {
    const ip = getLocalIP();
    expect(typeof ip).toBe('string');
    expect(ip.length).toBeGreaterThan(0);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd desktop && npx jest __tests__/server.test.js`
Expected: FAIL — startServer not defined

- [ ] **Step 3: Write server.js implementation**

```js
// desktop/server.js
const express = require('express');
const http = require('http');
const { WebSocketServer } = require('ws');
const path = require('path');
const os = require('os');
const { typeText } = require('./keyboard');

function getLocalIP() {
  const interfaces = os.networkInterfaces();
  for (const name of Object.keys(interfaces)) {
    for (const iface of interfaces[name]) {
      if (iface.family === 'IPv4' && !iface.internal) {
        return iface.address;
      }
    }
  }
  return '127.0.0.1';
}

function startServer(port = 9527) {
  const app = express();

  app.use(express.static(path.join(__dirname, 'public')));

  const server = http.createServer(app);
  const wss = new WebSocketServer({ server });

  wss.on('connection', (ws) => {
    ws.on('message', async (data) => {
      try {
        const msg = JSON.parse(data.toString());
        if (msg.type === 'text' && msg.payload != null) {
          await typeText(msg.payload);
          ws.send(JSON.stringify({ type: 'ack', payload: 'ok' }));
        } else if (msg.type === 'ping') {
          ws.send(JSON.stringify({ type: 'pong' }));
        }
      } catch {
        // ignore malformed messages
      }
    });
  });

  server.listen(port, () => {
    const ip = getLocalIP();
    console.log(`Server: http://${ip}:${port}`);
  });

  return { server, port };
}

module.exports = { startServer, getLocalIP };
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd desktop && npx jest __tests__/server.test.js`
Expected: PASS (6 tests)

- [ ] **Step 5: Commit**

```bash
git add desktop/server.js desktop/__tests__/server.test.js
git commit -m "feat: add HTTP + WebSocket server with text forwarding"
```

---

### Task 4: Phone web page

**Files:**
- Create: `desktop/public/index.html`

- [ ] **Step 1: Write the phone page**

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no">
  <title>语音输入</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
      background: #1a1a2e;
      color: #eee;
      display: flex;
      flex-direction: column;
      height: 100vh;
      height: 100dvh;
      padding: 16px;
    }
    .status-bar {
      display: flex; align-items: center; gap: 8px;
      padding: 12px 0;
      font-size: 14px;
      color: #aaa;
    }
    .dot {
      width: 10px; height: 10px;
      border-radius: 50%;
      flex-shrink: 0;
    }
    .dot.disconnected { background: #ff4444; }
    .dot.connecting  { background: #ffaa00; animation: pulse 1s infinite; }
    .dot.connected   { background: #44ff44; }
    @keyframes pulse {
      0%, 100% { opacity: 1; }
      50%      { opacity: 0.3; }
    }
    textarea {
      flex: 1; width: 100%;
      background: #16213e;
      color: #eee;
      border: 2px solid #0f3460;
      border-radius: 12px;
      padding: 16px;
      font-size: 18px;
      line-height: 1.6;
      resize: none;
      outline: none;
      -webkit-appearance: none;
    }
    textarea:focus {
      border-color: #e94560;
    }
    textarea::placeholder {
      color: #555;
    }
    .hint {
      text-align: center;
      color: #555;
      font-size: 12px;
      padding: 12px 0 4px;
    }
  </style>
</head>
<body>
  <div class="status-bar">
    <div class="dot disconnected" id="dot"></div>
    <span id="statusText">未连接</span>
  </div>

  <textarea id="input"
    placeholder="点击此处，切换到语音输入法开始说话..."
    autofocus></textarea>

  <p class="hint">说完后文字自动发送到电脑</p>

  <script>
    (function() {
      var dot = document.getElementById('dot');
      var statusText = document.getElementById('statusText');
      var textarea = document.getElementById('input');
      var ws = null;
      var timer = null;
      var retries = 0;
      var MAX_RETRIES = 10;

      function setStatus(state) {
        dot.className = 'dot ' + state;
        statusText.textContent = {
          disconnected: '未连接',
          connecting: '连接中...',
          connected: '已连接'
        }[state];
      }

      function connect() {
        if (ws && ws.readyState === WebSocket.OPEN) return;
        setStatus('connecting');

        var protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
        ws = new WebSocket(protocol + '//' + location.host);

        ws.onopen = function() {
          setStatus('connected');
          retries = 0;
          textarea.focus();
        };

        ws.onclose = function() {
          setStatus('disconnected');
          if (retries < MAX_RETRIES) {
            var delay = Math.min(1000 * Math.pow(2, retries), 30000);
            retries++;
            setTimeout(connect, delay);
          }
        };

        ws.onerror = function() {
          ws.close();
        };

        ws.onmessage = function(e) {
          try {
            var msg = JSON.parse(e.data);
            if (msg.type === 'ack') {
              // text delivered, could show brief visual feedback
            }
          } catch (ignore) {}
        };
      }

      textarea.addEventListener('input', function() {
        clearTimeout(timer);
        timer = setTimeout(function() {
          if (ws && ws.readyState === WebSocket.OPEN && textarea.value.trim()) {
            ws.send(JSON.stringify({ type: 'text', payload: textarea.value }));
          }
        }, 300);
      });

      // keepalive
      setInterval(function() {
        if (ws && ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ type: 'ping' }));
        }
      }, 30000);

      connect();
    })();
  </script>
</body>
</html>
```

- [ ] **Step 2: Verify the page is served**

Run: `node -e "const {startServer} = require('./desktop/server'); const s = startServer(19528); setTimeout(() => { const http = require('http'); http.get('http://localhost:19528/', r => { let d=''; r.on('data',c=>d+=c); r.on('end',()=>{console.log(d.includes('textarea')?'OK':'FAIL'); s.server.close(); process.exit(0); }); }); }, 500);"`

Expected: OK

- [ ] **Step 3: Commit**

```bash
git add desktop/public/index.html
git commit -m "feat: add phone web page with auto-send and reconnection"
```

---

### Task 5: QR code page

**Files:**
- Create: `desktop/public/qr.html`

- [ ] **Step 1: Add QR data API endpoint to server.js**

Add this code to `desktop/server.js` **after** `app.use(express.static(...))` and **before** `const server = http.createServer(app)`:

```js
  const QRCode = require('qrcode');

  app.get('/api/qr', async (_req, res) => {
    try {
      const ip = getLocalIP();
      const url = `http://${ip}:${port}`;
      const dataUrl = await QRCode.toDataURL(url);
      res.json({ url, dataUrl });
    } catch {
      res.status(500).json({ error: 'QR generation failed' });
    }
  });
```

And add this line at the top of `server.js` with the other requires:

```js
const QRCode = require('qrcode');
```

- [ ] **Step 2: Write the QR page**

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>扫码连接</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
      background: #1a1a2e;
      color: #eee;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100vh;
      padding: 24px;
    }
    h2 { font-size: 16px; margin-bottom: 16px; color: #ccc; }
    #qr-img { width: 260px; height: 260px; border-radius: 12px; background: #16213e; }
    .url {
      margin-top: 16px; font-size: 13px; color: #888;
      word-break: break-all; text-align: center;
      max-width: 280px;
      user-select: all;
    }
    .error { color: #e94560; font-size: 14px; }
  </style>
</head>
<body>
  <h2>📱 手机扫码连接</h2>
  <img id="qr-img" src="" alt="QR Code">
  <p class="url" id="url-text"></p>
  <p class="error" id="error" style="display:none"></p>

  <script>
    fetch('/api/qr')
      .then(function(r) { return r.json(); })
      .then(function(data) {
        document.getElementById('qr-img').src = data.dataUrl;
        document.getElementById('url-text').textContent = data.url;
      })
      .catch(function(err) {
        document.getElementById('error').style.display = 'block';
        document.getElementById('error').textContent = '生成二维码失败: ' + err.message;
      });
  </script>
</body>
</html>
```

- [ ] **Step 3: Verify QR API works**

Run: `node -e "const {startServer} = require('./desktop/server'); const s = startServer(19529); setTimeout(() => { const http = require('http'); http.get('http://localhost:19529/api/qr', r => { let d=''; r.on('data',c=>d+=c); r.on('end',()=>{const j=JSON.parse(d); console.log(j.url && j.dataUrl ? 'OK ' + j.url : 'FAIL'); s.server.close(); process.exit(0); }); }); }, 500);"`

Expected: OK http://...

- [ ] **Step 4: Commit**

```bash
git add desktop/server.js desktop/public/qr.html
git commit -m "feat: add QR code page and API endpoint"
```

---

### Task 6: Electron main process + system tray

**Files:**
- Create: `desktop/main.js`

- [ ] **Step 1: Write main.js**

```js
// desktop/main.js
const { app, Tray, Menu, BrowserWindow, nativeImage } = require('electron');
const path = require('path');
const { startServer, getLocalIP } = require('./server');

let tray = null;
let qrWindow = null;
let serverInfo = null;
let connected = false;

function createTrayIcon() {
  // Generate a 16x16 PNG: simple filled circle (greenish accent)
  // 1-pixel PNG with manual RGBA data
  const size = 16;
  const buf = Buffer.alloc(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const i = (y * size + x) * 4;
      const cx = 7.5, cy = 7.5, r = 6;
      const dist = Math.sqrt((x - cx) ** 2 + (y - cy) ** 2);
      if (dist <= r) {
        buf[i] = 0xe9;     // R
        buf[i + 1] = 0x45; // G
        buf[i + 2] = 0x60; // B
        buf[i + 3] = 255;  // A
      }
    }
  }
  return nativeImage.createFromBuffer(buf, { width: size, height: size });
}

function updateMenu() {
  const menu = Menu.buildFromTemplate([
    {
      label: '显示二维码',
      click: showQRWindow
    },
    {
      type: 'separator'
    },
    {
      label: connected ? '状态: 已连接' : '状态: 等待连接',
      enabled: false
    },
    {
      type: 'separator'
    },
    {
      label: '开机自启',
      type: 'checkbox',
      checked: app.getLoginItemSettings().openAtLogin,
      click: (menuItem) => {
        app.setLoginItemSettings({ openAtLogin: menuItem.checked });
      }
    },
    {
      type: 'separator'
    },
    {
      label: '退出',
      click: () => {
        app.quit();
      }
    }
  ]);

  tray.setContextMenu(menu);
  tray.setToolTip(connected ? '语音输入 - 已连接' : '语音输入 - 等待连接');
}

function showQRWindow() {
  if (qrWindow) {
    qrWindow.show();
    qrWindow.focus();
    return;
  }

  qrWindow = new BrowserWindow({
    width: 340,
    height: 420,
    title: '扫码连接',
    resizable: false,
    autoHideMenuBar: true,
    webPreferences: {
      nodeIntegration: false
    }
  });

  const port = serverInfo.port;
  qrWindow.loadURL(`http://localhost:${port}/qr.html`);

  qrWindow.on('close', (e) => {
    e.preventDefault();
    qrWindow.hide();
  });
}

app.whenReady().then(() => {
  // Start server
  serverInfo = startServer(9527);

  // Wait for server to be ready
  serverInfo.server.on('listening', () => {
    // Create tray
    const icon = createTrayIcon();
    tray = new Tray(icon);
    updateMenu();

    // Monitor connections for status updates
    const { WebSocketServer } = require('ws');
    // HACK: access internal wss from server to count clients
    setInterval(() => {
      const wasConnected = connected;
      const wss = serverInfo.server._wss;
      const clientCount = wss ? [...wss.clients].filter(c => c.readyState === 1).length : 0;
      connected = clientCount > 0;
      if (connected !== wasConnected) {
        updateMenu();
      }
    }, 1000);
  });

  // Show QR window on first launch
  showQRWindow();
});

// Don't quit when all windows are closed (keep tray alive)
app.on('window-all-closed', (e) => {
  e.preventDefault();
});

app.on('before-quit', () => {
  if (serverInfo && serverInfo.server) {
    serverInfo.server.close();
  }
});
```

- [ ] **Step 2: Verify Electron app starts**

Run: `cd desktop && npx electron .`
Expected: App starts, tray icon appears, QR window opens showing QR code.

- [ ] **Step 3: Commit**

```bash
git add desktop/main.js
git commit -m "feat: add Electron main process with system tray and QR window"
```

---

### Task 7: Final integration and README

**Files:**
- Create: `README.md`
- Modify: `desktop/server.js` — track WebSocket clients for status

- [ ] **Step 1: Fix server to expose connected client count**

The periodic check in `main.js` uses an internal property. Add proper tracking to `server.js`.

Add these lines to `server.js` inside `startServer()`, after the `wss` is created:

```js
  // Attach wss to server for external access
  server._wss = wss;
```

- [ ] **Step 2: Write README**

```markdown
# Voice Input

使用手机作为台式机语音输入源。手机说话 → 台式机实时输入文字。

## 使用方式

1. 台式机启动应用：`cd desktop && npm start`
2. 弹出二维码窗口，手机扫码
3. 手机浏览器打开页面，点击输入框
4. 切换到语音输入法（如豆包）开始说话
5. 文字自动出现在台式机光标所在位置

## 要求

- Node.js >= 18
- 手机和台式机在同一局域网
- Windows 10+（macOS 下键盘模拟需辅助功能权限）

## 初始化

```bash
cd desktop
npm install
npm start
```
```

- [ ] **Step 3: Commit**

```bash
git add README.md desktop/server.js
git commit -m "docs: add README and finalize server integration"
```

---

### Task 8: End-to-end manual test

- [ ] **Step 1: Start the app**

Run: `cd desktop && npm start`

- [ ] **Step 2: Verify QR code appears**

Expected: QR code window opens with scannable QR code.

- [ ] **Step 3: Scan QR with phone, verify page loads**

Expected: Phone browser opens the text input page, status shows "已连接".

- [ ] **Step 4: Type text on phone, verify it appears on desktop**

Steps:
1. Click the textarea on phone
2. Switch to voice input method (豆包 or any keyboard)
3. Speak a sentence
4. After ~300ms pause, the text should appear wherever the desktop cursor is

- [ ] **Step 5: Test reconnection**

Steps:
1. Close phone browser tab
2. Status dot turns red
3. Re-open page — should auto-reconnect to cached WebSocket

- [ ] **Step 6: Test tray menu**

Steps:
1. Right-click tray icon
2. "显示二维码" shows QR window
3. "开机自启" checkbox toggles login item
4. "退出" quits the app
