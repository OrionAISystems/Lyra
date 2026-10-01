import { deflateSync } from "node:zlib";
import { describe, expect, it } from "vitest";
import { validatePng } from "./themes-release.mjs";

const pngSignature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const crcTable = (() => {
  const table = new Uint32Array(256);
  for (let value = 0; value < table.length; value += 1) {
    let crc = value;
    for (let bit = 0; bit < 8; bit += 1) crc = (crc & 1) ? (0xedb88320 ^ (crc >>> 1)) : (crc >>> 1);
    table[value] = crc >>> 0;
  }
  return table;
})();

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) crc = crcTable[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const typeBytes = Buffer.from(type, "ascii");
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const checksum = Buffer.alloc(4);
  checksum.writeUInt32BE(crc32(Buffer.concat([typeBytes, data])), 0);
  return Buffer.concat([length, typeBytes, data, checksum]);
}

function makeRgba16Png(width, height) {
  const rowBytes = width * 4 * 2;
  const scanlines = Buffer.alloc((rowBytes + 1) * height);
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 16;
  header[9] = 6;
  const imageData = deflateSync(scanlines);
  return Buffer.concat([
    pngSignature,
    chunk("IHDR", header),
    chunk("IDAT", imageData),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

describe("themes-release PNG validation", () => {
  it("matches the runtime decoded-image limit", () => {
    expect(validatePng(makeRgba16Png(1024, 512), "limit PNG")).toEqual({ width: 1024, height: 512 });
    expect(() => validatePng(makeRgba16Png(1024, 1024), "oversized PNG"))
      .toThrow(/decoded pixel buffer is 8388608 bytes; the limit is 4194304/);
  });
});
