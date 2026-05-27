// Generate a 256x256 PNG icon for typelink — a stylized "T" on gradient background
const zlib = require('zlib');
const fs = require('fs');
const path = require('path');

const SIZE = 256;

// Build RGBA pixel data — row-major, each row filtered with byte 0 (None)
function buildImageData() {
  const rawRows = [];
  for (let y = 0; y < SIZE; y++) {
    const row = Buffer.alloc(1 + SIZE * 4); // filter byte + RGBA per pixel
    row[0] = 0; // filter: None
    for (let x = 0; x < SIZE; x++) {
      const offset = 1 + x * 4;

      // Background: dark gradient
      const r = Math.round(30 + (y / SIZE) * 20);
      const g = Math.round(30 + (y / SIZE) * 20);
      const b = Math.round(45 + (y / SIZE) * 25);
      let alpha = 255;

      // "TL" letter shape — geometric, side-by-side
      const inTBar = x >= 30 && x < 130 && y >= 58 && y < 94;
      const inTStem = x >= 62 && x < 98 && y >= 94 && y < 198;
      const inLStem = x >= 140 && x < 176 && y >= 58 && y < 198;
      const inLBar = x >= 140 && x < 226 && y >= 162 && y < 198;
      const inLetter = inTBar || inTStem || inLStem || inLBar;

      if (inLetter) {
        // White letter with slight transparency at edges
        let edgeDist = 999;
        if (inTBar) edgeDist = Math.min(edgeDist, x - 30, 130 - x, y - 58, 94 - y);
        if (inTStem) edgeDist = Math.min(edgeDist, x - 62, 98 - x, y - 94, 198 - y);
        if (inLStem) edgeDist = Math.min(edgeDist, x - 140, 176 - x, y - 58, 198 - y);
        if (inLBar) edgeDist = Math.min(edgeDist, x - 140, 226 - x, y - 162, 198 - y);
        const softness = Math.min(1, edgeDist / 4);
        alpha = Math.round(220 + 35 * softness);
        row[offset] = 230;
        row[offset + 1] = 230;
        row[offset + 2] = 240;
      } else {
        row[offset] = r;
        row[offset + 1] = g;
        row[offset + 2] = b;
      }
      row[offset + 3] = alpha;
    }
    rawRows.push(row);
  }
  return Buffer.concat(rawRows);
}

function crc32(buf) {
  let c;
  const table = [];
  for (let n = 0; n < 256; n++) {
    c = n;
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[n] = c;
  }
  c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) {
    c = table[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  }
  return (c ^ 0xffffffff) >>> 0;
}

function pngChunk(type, data) {
  const typeData = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(typeData), 0);
  return Buffer.concat([len, typeData, crc]);
}

const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

const ihdrData = Buffer.alloc(13);
ihdrData.writeUInt32BE(SIZE, 0);  // width
ihdrData.writeUInt32BE(SIZE, 4);  // height
ihdrData[8] = 8;   // bit depth
ihdrData[9] = 6;   // color type: RGBA
ihdrData[10] = 0;  // compression
ihdrData[11] = 0;  // filter
ihdrData[12] = 0;  // interlace

const rawData = buildImageData();
const compressed = zlib.deflateSync(rawData);

const png = Buffer.concat([
  signature,
  pngChunk('IHDR', ihdrData),
  pngChunk('IDAT', compressed),
  pngChunk('IEND', Buffer.alloc(0)),
]);

const outPath = path.join(__dirname, '..', 'assets', 'icon.png');
fs.writeFileSync(outPath, png);
console.log('Icon generated:', outPath);
