import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const bytes = await readFile(new URL("../dist/yydungeon.wasm", import.meta.url));
const decoder = new TextDecoder();
let wasm;
let draws = 0;
let sprites = 0;
const spritesDrawn = [];
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
  canvas_sprite(id, x, y, width, height) {
    assert.ok(id > 0 && id <= sprites);
    spritesDrawn.push({ id, x, y, width, height });
  },
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
assert.ok(labels.has("はじめから　 [Enter]"));
assert.equal(wasm.yy_can_autosave(), 0);
assert.equal(game().countdown, 42);
assert.equal(sprites, 79);
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
assert.ok([...labels].some(label => label.includes("魔王 Lv.1　次Lvまで 10 XP　次の階 Lv.3")));

wasm.yy_event(2, 51, 0); // 3 opens the monster roster.
wasm.yy_frame(0, 1);
assert.ok(labels.has("仲間モンスター図鑑") && [...labels].some(label => label.includes("スライム") && label.includes("D Lv.")));
for (let i = 0; i < 3; i++) wasm.yy_event(1, 420, 671);
wasm.yy_frame(0, 1);
assert.ok([...labels].some(label => label.includes("深淵竜") && label.includes("S Lv.")));
wasm.yy_event(2, 27, 0);
wasm.yy_frame(0, 1);

// Mouse placement through the shared screen, then keyboard starts the wave.
wasm.yy_event(1, 980, 350);
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
for (let i = 0; i < 600 && game().raid; i++) {
  const duel = game().raid.duel;
  if (duel) {
    const key = duel.outcome ? 13 : duel.boss_mp >= 3 ? 50 : 49;
    wasm.yy_event(2, key, 0);
  }
  gaugeColors.clear();
  wasm.yy_frame(0.25, 1);
  if (game().raid?.invaders.some(invader => invader.encounter)) {
    fightFrames++;
    assert.ok(gaugeColors.has("81c9b6") && gaugeColors.has("f39682"));
  }
}
assert.ok(fightFrames >= 5);
assert.ok(labels.has("執事 セヴラン"));
wasm.yy_event(2, 27, 0);
wasm.yy_frame(0, 1);
wasm.yy_event(1, 928, 48);
wasm.yy_frame(0, 1);
assert.ok(labels.has("魔族の秘書 レイラ"));
wasm.yy_event(2, 27, 0);
wasm.yy_frame(0, 1);
const played = game();
assert.equal(played.raid, null);
assert.ok(played.kills >= 1);
assert.ok(played.boss > 0);

const saveSize = wasm.yy_save_size();
const saved = new Uint8Array(wasm.memory.buffer, wasm.yy_save_ptr(), saveSize).slice();
const loadPtr = wasm.yy_load_buffer(saved.length);
new Uint8Array(wasm.memory.buffer, loadPtr, saved.length).set(saved);
wasm.yy_init(1240, 780);
labels.clear();
wasm.yy_frame(0.25, 1);
assert.ok(labels.has("つづきから　 [Enter]") && labels.has("はじめから"));
wasm.yy_event(1, 220, 520);
wasm.yy_frame(0, 1);
assert.ok(labels.has("最初から始める"));
assert.deepEqual(game(), played);
wasm.yy_event(1, 386, 480); // Cancel, then continue the existing save.
wasm.yy_frame(0, 1);
assert.deepEqual(game(), played);
assert.equal(wasm.yy_can_autosave(), 0);
assert.deepEqual(game(), played);
wasm.yy_event(2, 13, 0);
wasm.yy_frame(0, 1);
assert.equal(wasm.yy_can_autosave(), 1);
assert.deepEqual(game(), played);
console.log("Wasm renders, accepts construction orders, resolves a raid and resumes the saved game.");

// Select a new trap from the workshop catalogue.
wasm.yy_event(2, 57, 0);
wasm.yy_frame(0, 1);
assert.ok(labels.has("毒霧の罠 Lv.1") && labels.has("爆炎の罠 Lv.1") && labels.has("呪印の罠 Lv.1"));
wasm.yy_event(1, 540, 390);
wasm.yy_event(1, 289, 261);
wasm.yy_frame(0, 1);
assert.ok(game().floors[0].sites.some(site => site?.Trap?.kind === "Poison"));

// Dig the wall at (3, 3) and recruit through the roster.
wasm.yy_event(2, 54, 0);
wasm.yy_event(1, 289, 352);
wasm.yy_frame(0, 1);
assert.ok(game().development >= 3);
wasm.yy_event(2, 51, 0);
wasm.yy_event(1, 270, 258);
wasm.yy_event(1, 335, 352);
wasm.yy_frame(0, 1);
assert.ok(game().floors[0].sites.some(site => site?.Monster?.id === 0));

spritesDrawn.length = 0;
wasm.yy_event(2, 56, 0); // 8 places a rock on an unused passage.
wasm.yy_event(1, 244, 261);
wasm.yy_frame(0, 1);
assert.ok(game().floors[0].placed_rocks.some(cell => cell.x === 2 && cell.y === 1));
for (const label of ["掘削できる岩", "設置した岩壁", "掘削できない岩"]) {
  assert.ok(labels.has(label));
}
const rockSpriteAt = (x, y) => spritesDrawn.find(sprite =>
  sprite.x >= x && sprite.x < x + 46 && sprite.y >= y && sprite.y < y + 46 && sprite.width >= 42
)?.id;
const builtRock = rockSpriteAt(221, 237);
const diggableRock = rockSpriteAt(267, 283);
const solidRock = rockSpriteAt(129, 191);
assert.ok(builtRock && diggableRock && solidRock);
assert.equal(new Set([builtRock, diggableRock, solidRock]).size, 3);
const developmentBefore = game().development;
const goldBefore = game().gold;
wasm.yy_event(2, 55, 0);
wasm.yy_event(1, 244, 261);
wasm.yy_frame(0, 1);
assert.equal(game().floors[0].placed_rocks.length, 0);
assert.equal(game().development, developmentBefore);
assert.equal(game().gold, goldBefore - 8);

// Resume a saved throne fight, choose a spell, and return to the raid.
const arenaGame = game();
arenaGame.wave = 2;
arenaGame.raid = {
  tally: { wave: 2, stopped: 0, fled: 0, throne: 1, gold: 0, bones: 0, loot: 0, chests: 0 },
  duel: { class: 7, level: 4, hp: 15, max_hp: 62, mp: 6, max_mp: 6, boss_mp: 9, turn: 0, lines: [], outcome: null },
  waiting: [], invaders: [], clock: 0, spawn: 0, fallen: [],
};
const arenaSave = new TextEncoder().encode(JSON.stringify([1, arenaGame]));
const arenaPtr = wasm.yy_load_buffer(arenaSave.length);
new Uint8Array(wasm.memory.buffer, arenaPtr, arenaSave.length).set(arenaSave);
wasm.yy_init(1240, 780);
wasm.yy_event(2, 13, 0);
wasm.yy_frame(0, 1);
assert.ok(labels.has("玉座の戦い"));
const idleEnemyId = spritesDrawn.at(-1).id;
const idleBossId = spritesDrawn.at(-2).id;
const bossSprite = spritesDrawn.at(-2);
const adventurerSprite = spritesDrawn.at(-1);
assert.ok(bossSprite.x < 350 && bossSprite.y >= 195);
assert.ok(adventurerSprite.x > 700 && adventurerSprite.y < bossSprite.y);
assert.ok(adventurerSprite.height >= 365);
assert.ok(adventurerSprite.y + adventurerSprite.height < bossSprite.y + bossSprite.height);
wasm.yy_event(2, 50, 0);
wasm.yy_frame(0, 1);
assert.equal(game().raid.duel.outcome, "Victory");
assert.equal(game().raid.duel.boss_mp, 6);
assert.notEqual(spritesDrawn.at(-2).id, idleBossId); // Dark spell pose
assert.equal(spritesDrawn.at(-1).id, idleEnemyId);
wasm.yy_event(2, 13, 0);
wasm.yy_frame(0.25, 1);
assert.ok(game().raid.duel); // Presentation cannot be skipped by another input.
wasm.yy_frame(0.25, 1);
assert.notEqual(spritesDrawn.at(-1).id, idleEnemyId); // Impact pose
for (let i = 0; i < 6; i++) wasm.yy_frame(0.25, 1);
assert.ok(labels.has("攻防中") && labels.has("次へ　[Enter]"));
assert.ok(labels.has("獲得経験値 +10 XP"));
wasm.yy_event(2, 13, 0);
wasm.yy_frame(0.25, 1);
wasm.yy_frame(0.25, 1);
assert.equal(game().last_report.stopped, 1);
assert.equal(game().last_report.throne, 1);
assert.equal(game().last_report.boss_xp, 10);
assert.ok(game().boss_xp > 0);

// Both fighters get distinct charge, special, and recoil poses in the same turn.
arenaGame.floors.push(structuredClone(arenaGame.floors[0]));
arenaGame.floors.at(-1).tier = 2;
arenaGame.boss_xp = 42; // Lv. 4 unlocks Storm.
arenaGame.raid.duel = {
  class: 11, level: 7, hp: 100, max_hp: 100, mp: 9, max_mp: 9,
  boss_mp: 15, turn: 0, lines: [], outcome: null,
};
const specialSave = new TextEncoder().encode(JSON.stringify([1, arenaGame]));
new Uint8Array(wasm.memory.buffer, wasm.yy_load_buffer(specialSave.length), specialSave.length).set(specialSave);
wasm.yy_init(1240, 780);
wasm.yy_event(2, 13, 0);
wasm.yy_frame(0, 1);
const idlePair = spritesDrawn.slice(-2).map(sprite => sprite.id);
wasm.yy_event(2, 52, 0);
wasm.yy_frame(0, 1);
const chargePair = spritesDrawn.slice(-2).map(sprite => sprite.id);
assert.notEqual(chargePair[0], idlePair[0]);
for (let i = 0; i < 2; i++) wasm.yy_frame(0.25, 1);
const bossImpact = spritesDrawn.slice(-2).map(sprite => sprite.id);
assert.notEqual(bossImpact[0], chargePair[0]);
assert.notEqual(bossImpact[1], idlePair[1]);
assert.notEqual(bossImpact[1], chargePair[1]);
for (let i = 0; i < 2; i++) wasm.yy_frame(0.25, 1);
const enemyCharge = spritesDrawn.slice(-2).map(sprite => sprite.id);
assert.notEqual(enemyCharge[1], bossImpact[1]);
wasm.yy_frame(0.25, 1);
const enemyImpact = spritesDrawn.slice(-2).map(sprite => sprite.id);
assert.notEqual(enemyImpact[0], idlePair[0]);
assert.notEqual(enemyImpact[0], bossImpact[0]);
assert.notEqual(enemyImpact[1], enemyCharge[1]);
assert.ok([...labels].some(label => label.includes("光雷の魔法")));

// Confirm a new game from a title screen with an existing run.
const restartSize = wasm.yy_save_size();
const restartSave = new Uint8Array(wasm.memory.buffer, wasm.yy_save_ptr(), restartSize).slice();
const restartPtr = wasm.yy_load_buffer(restartSave.length);
new Uint8Array(wasm.memory.buffer, restartPtr, restartSave.length).set(restartSave);
wasm.yy_init(1240, 780);
wasm.yy_event(1, 220, 520);
wasm.yy_frame(0, 1);
assert.equal(wasm.yy_can_autosave(), 0);
wasm.yy_event(1, 835, 481);
wasm.yy_frame(0, 1);
assert.equal(wasm.yy_can_autosave(), 1);
assert.equal(game().wave, 0);
assert.equal(game().boss_xp, 0);
assert.equal(game().floors.length, 1);
