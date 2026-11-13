import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const bytes = await readFile(new URL("../dist/yydungeon.wasm", import.meta.url));
const decoder = new TextDecoder();
let wasm;
let draws = 0;
let sprites = 0;
const gaugeColors = new Set();
const labels = new Set();
const env = {
  canvas_begin() {},
  canvas_origin() {},
  canvas_rect(_x, _y, width, height, stroke, red, green, blue) {
    draws++;
    if (width > 0 && height === 3 && stroke === 0) {
      const hex = [red, green, blue].map(channel => Math.round(channel * 255).toString(16).padStart(2, "0")).join("");
      gaugeColors.add(hex);
    }
  },
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
assert.equal(sprites, 59);
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

wasm.yy_event(2, 51, 0); // 3 opens the monster roster.
wasm.yy_frame(0, 1);
assert.ok(labels.has("仲間モンスター図鑑") && labels.has("スライム"));
for (let i = 0; i < 3; i++) wasm.yy_event(1, 420, 671);
wasm.yy_frame(0, 1);
assert.ok(labels.has("深淵竜"));
wasm.yy_event(2, 27, 0);
wasm.yy_frame(0, 1);

// Mouse placement through the shared screen, then keyboard starts the wave.
wasm.yy_event(1, 980, 387);
wasm.yy_event(1, 198, 261);
wasm.yy_frame(0, 1);
assert.equal(game().gold, 151);
wasm.yy_event(2, 13, 0);
wasm.yy_frame(0, 1);
assert.equal(game().wave, 1);
wasm.yy_event(0, 151, 352);
wasm.yy_frame(0, 1);
assert.ok([...labels].some(label => label.includes("チンピラ") && label.includes("HP")));
let fightFrames = 0;
for (let i = 0; i < 200 && game().raid; i++) {
  gaugeColors.clear();
  wasm.yy_frame(0.25, 1);
  if (game().raid?.invaders.some(invader => invader.encounter)) {
    fightFrames++;
    assert.ok(gaugeColors.has("81c9b6") && gaugeColors.has("f39682"));
  }
}
assert.ok(fightFrames >= 5);
const played = game();
assert.equal(played.raid, null);
assert.equal(played.kills, 3);
assert.ok(played.boss > 0);

const saveSize = wasm.yy_save_size();
const saved = new Uint8Array(wasm.memory.buffer, wasm.yy_save_ptr(), saveSize).slice();
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

// Dig the wall at (3, 3) and select the newly unlocked mandrake from the roster.
wasm.yy_event(2, 54, 0);
wasm.yy_event(1, 289, 352);
wasm.yy_frame(0, 1);
assert.ok(game().development >= 3);
wasm.yy_event(2, 51, 0);
wasm.yy_event(1, 270, 425);
wasm.yy_event(1, 335, 352);
wasm.yy_frame(0, 1);
assert.ok(game().floors[0].sites.some(site => site?.Monster?.id === 4));

wasm.yy_event(2, 56, 0); // 8 places a rock on an unused passage.
wasm.yy_event(1, 244, 261);
wasm.yy_frame(0, 1);
assert.ok(game().floors[0].placed_rocks.some(cell => cell.x === 2 && cell.y === 1));
const developmentBefore = game().development;
wasm.yy_event(2, 54, 0);
wasm.yy_event(1, 244, 261);
wasm.yy_frame(0, 1);
assert.equal(game().floors[0].placed_rocks.length, 0);
assert.equal(game().development, developmentBefore);
