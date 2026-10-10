// Run with: node --experimental-vm-modules --test scripts/test-web-startup.mjs
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { createContext, SourceTextModule, SyntheticModule } from "node:vm";

const html = await readFile(new URL("../web/index.html", import.meta.url), "utf8");
const source = html.match(/<script type="module">([\s\S]*?)<\/script>/)?.[1];
assert.ok(source, "the page has a WASM bootstrap module");

// Execute the actual inline module, replacing only its WASM dependency and DOM.
async function bootstrap({ webgl = true, loseContext = true, probeError, initError, startError, waitForInit } = {}) {
  const calls = [];
  const hint = { textContent: "loading wasm" };
  const chrome = { hidden: true };
  const boot = {
    style: {},
    removed: false,
    messages: [],
    querySelector: () => hint,
    remove() { this.removed = true; },
    appendChild(message) { this.messages.push(message); },
  };
  const canvas = { focus: () => calls.push("focus") };
  const document = {
    getElementById: (id) => ({ boot, chrome })[id],
    querySelectorAll: () => [canvas],
    createElement(tag) {
      if (tag !== "canvas") return { style: {}, textContent: "" };
      return {
        getContext(name) {
          calls.push(name);
          if (probeError) throw probeError;
          return webgl ? {
            getExtension(name) {
              assert.equal(name, "WEBGL_lose_context");
              return loseContext ? { loseContext: () => calls.push("release") } : null;
            },
          } : null;
        },
      };
    },
  };
  const context = createContext({ document });
  const wasm = new SyntheticModule(["default", "start"], function () {
    this.setExport("default", async () => {
      calls.push("init");
      if (initError) throw initError;
      await waitForInit;
    });
    this.setExport("start", () => {
      calls.push("start");
      assert.equal(hint.textContent, "starting");
      if (startError) throw startError;
    });
  }, { context });
  const module = new SourceTextModule(source, { context });
  await module.link((specifier) => {
    assert.match(specifier, /^\.\/pkg\/geofront\.js(?:\?|$)/);
    return wasm;
  });
  const finished = module.evaluate();
  return { calls, hint, chrome, boot, canvas, finished };
}

test("the hidden attribute overrides the controls' flex display", () => {
  assert.match(html, /<div id="chrome" hidden>/);
  assert.match(html, /#chrome\[hidden\]\s*\{\s*display:\s*none\s*;\s*\}/);
});

test("missing WebGL2 shows guidance without initializing the game", async () => {
  const state = await bootstrap({ webgl: false });
  await state.finished;
  assert.deepEqual(state.calls, ["webgl2"]);
  assert.equal(state.chrome.hidden, true);
  assert.equal(state.boot.removed, false);
  assert.match(state.boot.messages[0].textContent, /could not create a WebGL2 graphics context/);
  assert.match(state.boot.messages[0].textContent, /graphics acceleration/);
  assert.match(state.boot.messages[0].textContent, /regular browser on your device/);
});

test("controls remain hidden while WASM loads and appear after startup", async () => {
  let resolveInit;
  const waitForInit = new Promise((resolve) => { resolveInit = resolve; });
  const state = await bootstrap({ waitForInit });
  assert.deepEqual(state.calls, ["webgl2", "release", "init"]);
  assert.equal(state.chrome.hidden, true);
  assert.equal(state.boot.removed, false);
  resolveInit();
  await state.finished;
  assert.deepEqual(state.calls, ["webgl2", "release", "init", "start", "focus"]);
  assert.equal(state.chrome.hidden, false);
  assert.equal(state.boot.removed, true);
  assert.equal(state.canvas.tabIndex, 0);
});

test("startup also works without the optional context-release extension", async () => {
  const state = await bootstrap({ loseContext: false });
  await state.finished;
  assert.deepEqual(state.calls, ["webgl2", "init", "start", "focus"]);
  assert.equal(state.chrome.hidden, false);
});

test("winit's expected control-flow exception still completes startup", async () => {
  const state = await bootstrap({ startError: new Error("Using exceptions for control flow") });
  await state.finished;
  assert.equal(state.chrome.hidden, false);
  assert.equal(state.boot.removed, true);
  assert.deepEqual(state.boot.messages, []);
  assert.equal(state.calls.at(-1), "focus");
});

for (const [option, message, calls] of [
  ["probeError", "Graphics access denied", ["webgl2"]],
  ["initError", "WASM download failed", ["webgl2", "release", "init"]],
  ["startError", "Shader compilation failed", ["webgl2", "release", "init", "start"]],
]) {
  test(`${option} preserves the original error and keeps controls hidden`, async () => {
    const error = new Error(message);
    const state = await bootstrap({ [option]: error });
    await state.finished;
    assert.deepEqual(state.calls, calls);
    assert.equal(state.chrome.hidden, true);
    assert.equal(state.boot.removed, false);
    assert.equal(state.boot.messages.length, 1);
    assert.equal(state.boot.messages[0].textContent, String(error));
  });
}
