// Run with Node.js and the sharp development tool available on NODE_PATH.
const fs = require("node:fs/promises");
const path = require("node:path");
const sharp = require("sharp");

const root = path.resolve(__dirname, "../..");
const sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256];

async function buildIcon(name, app) {
  const dest = path.join(root, app, "src-tauri/icons");
  const directory = Buffer.alloc(6 + sizes.length * 16);
  directory.writeUInt16LE(1, 2); // ICO type
  directory.writeUInt16LE(sizes.length, 4);
  const frames = [];
  let offset = directory.length;
  for (const [index, size] of sizes.entries()) {
    const source = path.join(root, "assets/src", name + (size <= 24 ? "-mini.svg" : ".svg"));
    const png = await sharp(source, { density: 288 }).resize(size, size).ensureAlpha().png().toBuffer();
    const entry = 6 + index * 16;
    directory[entry] = directory[entry + 1] = size % 256;
    directory.writeUInt16LE(1, entry + 4);
    directory.writeUInt16LE(32, entry + 6);
    directory.writeUInt32LE(png.length, entry + 8);
    directory.writeUInt32LE(offset, entry + 12);
    frames.push(png);
    offset += png.length;
    if (size === 32) await fs.writeFile(path.join(dest, "32x32.png"), png);
  }
  await fs.writeFile(path.join(dest, "icon.ico"), Buffer.concat([directory, ...frames]));
  if (name === "ice") {
    await sharp(path.join(root, "assets/src/ice-mini.svg"), { density: 288 })
      .resize(48, 48).ensureAlpha().png().toFile(path.join(dest, "tray.png"));
  }
}

Promise.all([buildIcon("ice", "app"), buildIcon("pandora", "pandora")]).catch(error => {
  console.error(error);
  process.exitCode = 1;
});
