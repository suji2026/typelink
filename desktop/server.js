const express = require('express');
const http = require('http');
const { WebSocketServer } = require('ws');
const path = require('path');
const os = require('os');
const { execSync } = require('child_process');
const { typeText } = require('./keyboard');
const QRCode = require('qrcode');

function getLocalIPs() {
  const interfaces = os.networkInterfaces();
  const ips = [];
  const virtualNetworks = [
    '192.168.137.',  // VirtualBox NAT
    '192.168.159.',  // VirtualBox Host-Only
    '192.168.59.',   // Docker
    '172.17.',       // Docker
    '172.18.',       // Docker
    '172.19.',       // Docker
    '172.20.',       // Docker
    '172.21.',       // Docker
  ];
  
  for (const name of Object.keys(interfaces)) {
    for (const iface of interfaces[name]) {
      if (iface.family === 'IPv4' && !iface.internal) {
        const ip = iface.address;
        
        const isVirtualIP = virtualNetworks.some(net => {
          if (net.endsWith('.')) {
            return ip.startsWith(net);
          }
          return false;
        });
        
        if (isVirtualIP) continue;
        
        if (ip.startsWith('192.168.') ||
            ip.startsWith('172.') || 
            ip.startsWith('10.')) {
          ips.push(ip);
        }
      }
    }
  }
  
  return ips;
}

function getLocalIP() {
  const ips = getLocalIPs();
  return ips.length > 0 ? ips[0] : '127.0.0.1';
}

function startServer(port = 9527) {
  const app = express();

  app.use(express.static(path.join(__dirname, 'public')));

  app.get('/api/qr', async (req, res) => {
    try {
      const ips = getLocalIPs();
      const selectedIP = req.query.ip || (ips.length > 0 ? ips[0] : '127.0.0.1');
      const url = `http://${selectedIP}:${port}`;
      const dataUrl = await QRCode.toDataURL(url);
      const clientCount = [...wss.clients].filter(c => c.readyState === 1).length;
      
      const allIPs = ips.map(ip => ({
        ip,
        name: 'LAN',
        label: ip.startsWith('192.168.137.') ? '移动热点 (Mobile Hotspot)' : '本地局域网 (LAN)'
      }));

      res.json({ 
        url, 
        dataUrl,
        connected: clientCount > 0,
        clientCount,
        allIPs
      });
    } catch {
      res.status(500).json({ error: 'QR generation failed' });
    }
  });

  const server = http.createServer(app);
  const wss = new WebSocketServer({ server });
  server._wss = wss;

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
    const ips = getLocalIPs();
    console.log(`Server running on port ${port}`);
    ips.forEach(ip => {
      console.log(`  - http://${ip}:${port}`);
    });
  });

  return { server, wss, port };
}

module.exports = { startServer, getLocalIP };