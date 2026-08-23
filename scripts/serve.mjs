#!/usr/bin/env node
/** Static file server for the WASM build. Binds 0.0.0.0:8080. */
import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, resolve } from "node:path";

const root = resolve(process.cwd(), process.argv[2] ?? "dist");
const extra = resolve(process.cwd(), "public");
const port = Number(process.env.PORT ?? 8080);
const mime = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".svg": "image/svg+xml",
  ".wgsl": "text/plain; charset=utf-8",
  ".ogg": "audio/ogg",
  ".mp3": "audio/mpeg",
};

function resolveFile(rel) {
  const candidates = [root, extra];
  for (const base of candidates) {
    const file = normalize(join(base, rel)).replace(/\\/g, "/");
    const baseNorm = base.replace(/\\/g, "/");
    if (!file.startsWith(baseNorm)) continue;
    if (existsSync(file) && statSync(file).isFile()) return file;
  }
  const fallback = join(root, "index.html");
  return existsSync(fallback) ? fallback : null;
}

const server = createServer((req, res) => {
  const url = new URL(req.url ?? "/", "http://127.0.0.1");
  let rel = decodeURIComponent(url.pathname);
  if (rel.endsWith("/")) rel += "index.html";
  const path = resolveFile(rel);
  if (!path) {
    res.writeHead(404, { "content-type": "text/plain" });
    res.end("build the wasm game first: bash scripts/build-web.sh");
    return;
  }
  res.writeHead(200, {
    "content-type": mime[extname(path)] ?? "application/octet-stream",
    "cache-control": "no-store",
  });
  createReadStream(path).pipe(res);
});

server.listen(port, "0.0.0.0", () => {
  console.log(`geofront ${root} -> 0.0.0.0:${port}`);
});
