const { keyboard, Key } = require('@nut-tree-fork/nut-js');
const os = require('os');

async function typeText(text) {
  const { default: clipboardy } = await import('clipboardy');
  
  await clipboardy.write(text);
  
  const platform = os.platform();
  const pasteKey = platform === 'darwin' ? Key.LeftSuper : Key.LeftControl;
  
  await keyboard.pressKey(pasteKey, Key.V);
  await keyboard.releaseKey(pasteKey, Key.V);
  
  await new Promise(resolve => setTimeout(resolve, 50));
}

module.exports = { typeText };
