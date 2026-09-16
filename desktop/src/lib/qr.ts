/**
 * Minimal QR code SVG generator for pairing payloads.
 * Uses qrcode-svg-style encoding via canvas-free approach.
 * ponytail: replace with proper library when bundle size allows.
 */

// Simple QR Code Model 2, Version 1-4 encoder (alphanumeric/byte mode)
// Generates SVG string directly — no DOM/canvas dependency.

// Galois Field arithmetic for Reed-Solomon
const GF256_EXP = new Uint8Array(512);
const GF256_LOG = new Uint8Array(256);
(() => {
  let x = 1;
  for (let i = 0; i < 255; i++) {
    GF256_EXP[i] = x;
    GF256_LOG[x] = i;
    x <<= 1;
    if (x & 0x100) x ^= 0x11d;
  }
  for (let i = 255; i < 512; i++) GF256_EXP[i] = GF256_EXP[i - 255];
})();

function gfMul(a: number, b: number): number {
  if (a === 0 || b === 0) return 0;
  return GF256_EXP[GF256_LOG[a] + GF256_LOG[b]];
}

function rsGenPoly(n: number): Uint8Array {
  let poly = new Uint8Array([1]);
  for (let i = 0; i < n; i++) {
    const next = new Uint8Array(poly.length + 1);
    for (let j = 0; j < poly.length; j++) {
      next[j] ^= poly[j];
      next[j + 1] ^= gfMul(poly[j], GF256_EXP[i]);
    }
    poly = next;
  }
  return poly;
}

function rsEncode(data: Uint8Array, ecLen: number): Uint8Array {
  const gen = rsGenPoly(ecLen);
  const result = new Uint8Array(ecLen);
  for (let i = 0; i < data.length; i++) {
    const factor = data[i] ^ result[0];
    for (let j = 0; j < ecLen - 1; j++) result[j] = result[j + 1] ^ gfMul(gen[j + 1], factor);
    result[ecLen - 1] = gfMul(gen[ecLen], factor);
  }
  return result;
}

// Version capacities (byte mode, EC level L)
const VERSION_CAPS = [
  null,
  { size: 21, dataCodewords: 19, ecPerBlock: 7, blocks: 1 },   // v1: 17 bytes
  { size: 25, dataCodewords: 34, ecPerBlock: 10, blocks: 1 },  // v2: 32 bytes
  { size: 29, dataCodewords: 55, ecPerBlock: 15, blocks: 1 },  // v3: 53 bytes
  { size: 33, dataCodewords: 80, ecPerBlock: 20, blocks: 1 },  // v4: 78 bytes
];

function pickVersion(dataLen: number): number {
  for (let v = 1; v <= 4; v++) {
    const cap = VERSION_CAPS[v]!;
    if (dataLen <= cap.dataCodewords - 2) return v; // -2 for mode+length header
  }
  throw new Error("QR payload too large (>78 bytes for v4-L)");
}

function encodeByteMode(text: string, version: number): Uint8Array {
  const bytes = new TextEncoder().encode(text);
  const bits: number[] = [];
  // Mode indicator: 0100 (byte mode)
  bits.push(0, 1, 0, 0);
  // Character count (8 bits for v1-9)
  for (let i = 7; i >= 0; i--) bits.push((bytes.length >> i) & 1);
  // Data
  for (const b of bytes) for (let i = 7; i >= 0; i--) bits.push((b >> i) & 1);
  // Terminator
  const cap = VERSION_CAPS[version]!.dataCodewords;
  const totalBits = cap * 8;
  for (let i = 0; i < Math.min(4, totalBits - bits.length); i++) bits.push(0);
  // Pad to byte boundary
  while (bits.length % 8 !== 0) bits.push(0);
  // Pad codewords
  const padBytes = [0xec, 0x11];
  let pi = 0;
  while (bits.length < totalBits) {
    const pb = padBytes[pi++ % 2];
    for (let i = 7; i >= 0; i--) bits.push((pb >> i) & 1);
  }
  // Convert to bytes
  const codewords = new Uint8Array(bits.length / 8);
  for (let i = 0; i < codewords.length; i++) {
    let val = 0;
    for (let j = 0; j < 8; j++) val = (val << 1) | bits[i * 8 + j];
    codewords[i] = val;
  }
  return codewords;
}

function buildMatrix(version: number, dataCodewords: Uint8Array, ecCodewords: Uint8Array): { modules: boolean[][]; size: number } {
  const cap = VERSION_CAPS[version]!;
  const size = cap.size;
  const modules: (boolean | null)[][] = Array.from({ length: size }, () => Array(size).fill(null));
  const reserved: boolean[][] = Array.from({ length: size }, () => Array(size).fill(false));

  // Finder patterns
  function setFinder(row: number, col: number) {
    for (let r = -1; r <= 7; r++) for (let c = -1; c <= 7; c++) {
      const rr = row + r, cc = col + c;
      if (rr < 0 || rr >= size || cc < 0 || cc >= size) continue;
      const isBlack = (r >= 0 && r <= 6 && c >= 0 && c <= 6) &&
        (r === 0 || r === 6 || c === 0 || c === 6 || (r >= 2 && r <= 4 && c >= 2 && c <= 4));
      modules[rr][cc] = isBlack;
      reserved[rr][cc] = true;
    }
  }
  setFinder(0, 0);
  setFinder(0, size - 7);
  setFinder(size - 7, 0);

  // Timing patterns
  for (let i = 8; i < size - 8; i++) {
    modules[6][i] = i % 2 === 0;
    modules[i][6] = i % 2 === 0;
    reserved[6][i] = true;
    reserved[i][6] = true;
  }

  // Dark module
  modules[size - 8][8] = true;
  reserved[size - 8][8] = true;

  // Reserve format info areas
  for (let i = 0; i < 8; i++) {
    reserved[8][i] = true;
    reserved[8][size - 1 - i] = true;
    reserved[i][8] = true;
    reserved[size - 1 - i][8] = true;
  }
  reserved[8][8] = true;

  // Place data
  const allData = new Uint8Array([...dataCodewords, ...ecCodewords]);
  let bitIdx = 0;
  let upward = true;
  for (let col = size - 1; col >= 0; col -= 2) {
    if (col === 6) col = 5; // skip timing column
    for (let row = 0; row < size; row++) {
      const actualRow = upward ? size - 1 - row : row;
      for (const dc of [0, -1]) {
        const c = col + dc;
        if (c < 0 || reserved[actualRow][c]) continue;
        if (bitIdx < allData.length * 8) {
          const byteIdx = Math.floor(bitIdx / 8);
          const bitOff = 7 - (bitIdx % 8);
          modules[actualRow][c] = ((allData[byteIdx] >> bitOff) & 1) === 1;
          bitIdx++;
        } else {
          modules[actualRow][c] = false;
        }
      }
    }
    upward = !upward;
  }

  // Apply mask pattern 0 (checkerboard: (row+col)%2==0)
  for (let r = 0; r < size; r++) for (let c = 0; c < size; c++) {
    if (!reserved[r][c] && modules[r][c] !== null) {
      if ((r + c) % 2 === 0) modules[r][c] = !modules[r][c]!;
    }
  }

  // Format info for mask 0, EC level L = 111011111000100
  const FORMAT_BITS = [1,1,1,0,1,1,1,1,1,0,0,0,1,0,0];
  // Horizontal strip around top-left finder
  const hPositions = [[8,0],[8,1],[8,2],[8,3],[8,4],[8,5],[8,7],[8,8],[7,8],[5,8],[4,8],[3,8],[2,8],[1,8],[0,8]];
  for (let i = 0; i < 15; i++) {
    const [r, c] = hPositions[i];
    modules[r][c] = FORMAT_BITS[i] === 1;
  }
  // Vertical strip
  const vPositions = [[size-1,8],[size-2,8],[size-3,8],[size-4,8],[size-5,8],[size-6,8],[size-7,8],[8,size-8],[8,size-7],[8,size-6],[8,size-5],[8,size-4],[8,size-3],[8,size-2],[8,size-1]];
  for (let i = 0; i < 15; i++) {
    const [r, c] = vPositions[i];
    modules[r][c] = FORMAT_BITS[i] === 1;
  }

  return { modules: modules as boolean[][], size };
}

export function generateQrSvg(text: string, pixelSize = 200): string {
  const version = pickVersion(new TextEncoder().encode(text).length);
  const cap = VERSION_CAPS[version]!;
  const dataCodewords = encodeByteMode(text, version);
  const ecCodewords = rsEncode(dataCodewords, cap.ecPerBlock);
  const qr = buildMatrix(version, dataCodewords, ecCodewords);

  const margin = 4;
  const totalSize = qr.size + margin * 2;
  const scale = pixelSize / totalSize;

  let rects = "";
  for (let r = 0; r < qr.size; r++) {
    for (let c = 0; c < qr.size; c++) {
      if (qr.modules[r][c]) {
        rects += `<rect x="${(c + margin) * scale}" y="${(r + margin) * scale}" width="${scale}" height="${scale}"/>`;
      }
    }
  }

  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${pixelSize} ${pixelSize}" width="${pixelSize}" height="${pixelSize}"><rect width="${pixelSize}" height="${pixelSize}" fill="white"/>${rects}</svg>`;
}
