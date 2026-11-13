import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const bytes = await readFile(new URL("../dist/yydungeon.wasm", import.meta.url));
const decoder = new TextDecoder();
let wasm;
let draws = 0;
let sprites = 0;
const labels = new Set();
const env = {
  canvas_begin() {},
  canvas_origin() {},
  canvas_rect() { draws++; },
  canvas_circle() {},
  canvas_text(ptr, len) { labels.add(decoder.decode(new Uint8Array(wasm.memory.buffer, ptr, len))); },
  canvas_bitmap() { return ++sprites; },
  canvas_sprite(id) { assert.ok(id > 0 && id <= sprites); },
  browser_fullscreen() {},
  browser_error(ptr, len) { throw new Error(decoder.decode(new Uint8Array(wasm.memory.buffer, ptr, len))); },
};
const game = () => {
  const size = wasm.yy_save_size();
  return JSON.parse(decoder.decode(new Uint8Array(wasm.memory.buffer, wasm.yy_save_ptr(), size)))[1];
};
wasm = (await WebAssembly.instantiate(bytes, { env })).instance.exports;
wasm.yy_init(1240, 780);
wasm.yy_frame(0.25, 1);
assert.ok(labels.has("はじめる　 [Enter]"));
assert.equal(wasm.yy_can_autosave(), 0);
assert.equal(game().countdown, 42);
assert.equal(sprites, 12);
wasm.yy_event(2, 112, 0);
wasm.yy_frame(0, 1);
assert.ok(labels.has("キャッシュを消して再読み込み"));
wasm.yy_event(1, 620, 595);
assert.equal(wasm.yy_frame(0, 1), 1);
assert.equal(wasm.yy_frame(0, 1), 0);
assert.equal(wasm.yy_can_autosave(), 0);
wasm.yy_event(2, 27, 0);
wasm.yy_event(1, 220, 460);
wasm.yy_frame(0, 1);
assert.equal(wasm.yy_can_autosave(), 1);
assert.equal(game().wave, 0);
assert.ok(draws > 100);

// Mouse placement through the shared screen, then keyboard starts the wave.
wasm.yy_event(1, 980, 406);
wasm.yy_event(1, 198, 307);
wasm.yy_frame(0, 1);
assert.equal(game().gold, 151);
wasm.yy_event(2, 13, 0);
wasm.yy_frame(0, 1);
assert.equal(game().wave, 1);
for (let i = 0; i < 140 && game().raid; i++) wasm.yy_frame(0.25, 1);
const played = game();
assert.equal(played.raid, null);
assert.equal(played.kills, 3);
assert.ok(played.boss > 0);

const saved = new Uint8Array(wasm.memory.buffer, wasm.yy_save_ptr(), wasm.yy_save_size()).slice();
const loadPtr = wasm.yy_load_buffer(saved.length);
new Uint8Array(wasm.memory.buffer, loadPtr, saved.length).set(saved);
wasm.yy_init(1240, 780);
labels.clear();
wasm.yy_frame(0.25, 1);
assert.ok(labels.has("つづきから　 [Enter]"));
assert.equal(wasm.yy_can_autosave(), 0);
assert.deepEqual(game(), played);
wasm.yy_event(2, 13, 0);
wasm.yy_frame(0, 1);
assert.equal(wasm.yy_can_autosave(), 1);
assert.deepEqual(game(), played);
console.log("Wasm renders, accepts construction orders, resolves a raid and resumes the saved game.");
