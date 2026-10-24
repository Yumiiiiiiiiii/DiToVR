import {crc32, inflateRawSync} from "node:zlib";

// Store submission accepts a ZIP containing only the new MSIX. No staging directory
// or credentials can accidentally enter this archive. MSIX is already compressed.
export function singleFileZip(fileName, data) {
  if (!/^[A-Za-z0-9][A-Za-z0-9_.-]+$/.test(fileName) || !Buffer.isBuffer(data) || data.length >= 0xffffffff) {
    throw new Error("Expected one plain filename and data smaller than 4 GiB.");
  }
  const name = Buffer.from(fileName);
  const local = Buffer.alloc(30);
  local.writeUInt32LE(0x04034b50, 0);
  local.writeUInt16LE(20, 4);
  local.writeUInt16LE(0x21, 12); // 1980-01-01, valid ZIP timestamp
  local.writeUInt32LE(crc32(data), 14);
  local.writeUInt32LE(data.length, 18);
  local.writeUInt32LE(data.length, 22);
  local.writeUInt16LE(name.length, 26);
  const central = Buffer.alloc(46);
  central.writeUInt32LE(0x02014b50, 0);
  central.writeUInt16LE(20, 4);
  local.copy(central, 6, 4, 28);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(1, 8);
  end.writeUInt16LE(1, 10);
  end.writeUInt32LE(central.length + name.length, 12);
  end.writeUInt32LE(local.length + name.length + data.length, 16);
  return Buffer.concat([local, name, data, central, name, end]);
}

// MakeAppx writes ZIP64 even for small MSIX files. Read only the requested
// manifest, with bounded offsets and output; never extract executable files.
export function zipEntry(zip, name) {
  function uint64(offset) {
    const value = zip.readBigUInt64LE(offset);
    if (value > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error("ZIP64 value exceeds safe bounds.");
    return Number(value);
  }
  let end = zip.length - 22;
  const lower = Math.max(0, end - 65535);
  while (end >= lower && zip.readUInt32LE(end) !== 0x06054b50) end--;
  if (end < lower || zip.length < 22 || end + 22 + zip.readUInt16LE(end + 20) !== zip.length) throw new Error("Invalid ZIP footer.");
  let count = zip.readUInt16LE(end + 10);
  let centralStart = zip.readUInt32LE(end + 16);
  let centralSize = zip.readUInt32LE(end + 12);
  let footerStart = end;
  if (end >= 20 && zip.readUInt32LE(end - 20) === 0x07064b50) {
    const locator = end - 20;
    if (zip.readUInt32LE(locator + 4) !== 0 || zip.readUInt32LE(locator + 16) !== 1) throw new Error("Multi-disk ZIP64 is unsupported.");
    const record = uint64(locator + 8);
    if (record + 56 > locator || zip.readUInt32LE(record) !== 0x06064b50 || record + 12 + uint64(record + 4) !== locator || zip.readUInt32LE(record + 16) !== 0 || zip.readUInt32LE(record + 20) !== 0 || uint64(record + 24) !== uint64(record + 32)) throw new Error("Invalid ZIP64 footer.");
    count = uint64(record + 32);
    centralSize = uint64(record + 40);
    centralStart = uint64(record + 48);
    footerStart = record;
  } else if (zip.readUInt32LE(end + 4) !== 0 || zip.readUInt16LE(end + 8) !== count || count === 0xffff || centralStart === 0xffffffff || centralSize === 0xffffffff) throw new Error("Unsupported ZIP layout.");
  let offset = centralStart;
  const centralEnd = offset + centralSize;
  if (count > 16384 || centralEnd > footerStart) throw new Error("Invalid ZIP directory.");
  const matches = [];
  for (let i = 0; i < count; i++) {
    if (offset + 46 > centralEnd || zip.readUInt32LE(offset) !== 0x02014b50) throw new Error("Invalid ZIP directory entry.");
    const nameLength = zip.readUInt16LE(offset + 28);
    const next = offset + 46 + nameLength + zip.readUInt16LE(offset + 30) + zip.readUInt16LE(offset + 32);
    if (next > centralEnd) throw new Error("Invalid ZIP entry bounds.");
    if (zip.toString("utf8", offset + 46, offset + 46 + nameLength) === name) matches.push(offset);
    offset = next;
  }
  if (offset !== centralEnd) throw new Error("ZIP directory length does not match its entry count.");
  if (matches.length !== 1) throw new Error("The MSIX must contain exactly one AppxManifest.xml.");
  offset = matches[0];
  let packedSize = zip.readUInt32LE(offset + 20);
  let size = zip.readUInt32LE(offset + 24);
  let local = zip.readUInt32LE(offset + 42);
  let disk = zip.readUInt16LE(offset + 34);
  if ([packedSize, size, local].includes(0xffffffff) || disk === 0xffff) {
    let extra = offset + 46 + zip.readUInt16LE(offset + 28);
    const extraEnd = extra + zip.readUInt16LE(offset + 30);
    let found = false;
    while (extra + 4 <= extraEnd) {
      const type = zip.readUInt16LE(extra), length = zip.readUInt16LE(extra + 2);
      if (extra + 4 + length > extraEnd) throw new Error("Invalid ZIP64 extra data.");
      if (type === 1) {
        let cursor = extra + 4;
        const take64 = () => { if (cursor + 8 > extra + 4 + length) throw new Error("Truncated ZIP64 entry."); const value = uint64(cursor); cursor += 8; return value; };
        if (size === 0xffffffff) size = take64();
        if (packedSize === 0xffffffff) packedSize = take64();
        if (local === 0xffffffff) local = take64();
        if (disk === 0xffff) { if (cursor + 4 > extra + 4 + length) throw new Error("Truncated ZIP64 disk."); disk = zip.readUInt32LE(cursor); }
        found = true;
        break;
      }
      extra += 4 + length;
    }
    if (!found) throw new Error("Missing ZIP64 entry data.");
  }
  if (disk !== 0) throw new Error("Multi-disk ZIP entries are unsupported.");
  if (size > 1024 * 1024 || packedSize > 1024 * 1024 || (zip.readUInt16LE(offset + 8) & 1) || local + 30 > zip.length || zip.readUInt32LE(local) !== 0x04034b50) throw new Error("Unsupported manifest ZIP entry.");
  const start = local + 30 + zip.readUInt16LE(local + 26) + zip.readUInt16LE(local + 28);
  if (start + packedSize > centralStart || zip.toString("utf8", local + 30, local + 30 + zip.readUInt16LE(local + 26)) !== name) throw new Error("Invalid manifest ZIP bounds or name.");
  const method = zip.readUInt16LE(offset + 10);
  const packed = zip.subarray(start, start + packedSize);
  const result = method === 0 ? packed : method === 8 ? inflateRawSync(packed, {maxOutputLength: 1024 * 1024}) : null;
  if (!result || result.length !== size || crc32(result) !== zip.readUInt32LE(offset + 16)) throw new Error("Invalid manifest ZIP checksum or compression.");
  return result;
}
