const canvas = document.getElementById("game");
const status = document.getElementById("status");
const portraitTouch = matchMedia("(orientation: portrait) and (pointer: coarse)");
const ctx = canvas.getContext("2d", { alpha: false });
const decoder = new TextDecoder();
const encoder = new TextEncoder();
const bitmaps = [];
const colorCache = new Map();
const saveKey = "yydungeon.save.v1";
const sans = '"Yu Gothic UI", "Hiragino Kaku Gothic ProN", Meiryo, "Noto Sans JP", sans-serif';
const serif = '"Yu Mincho", "Hiragino Mincho ProN", "Noto Serif JP", serif';
const fonts = new Map([
  [0, [12, false, '12px ' + sans]],
  [1, [14, false, '14px ' + sans]],
  [2, [14, true, '14px ' + sans]],
  [3, [18, false, 'bold 18px ' + sans]],
  [5, [40, true, 'bold 40px ' + serif]],
  [6, [30, true, 'bold 30px ' + serif]],
]);
let wasm;
let refreshing = false;

function memory(ptr, len) {
  return new Uint8Array(wasm.memory.buffer, ptr, len);
}

function color(r, g, b, a = 1) {
  const channels = [r, g, b, a].map(n => Math.round(n * 255));
  const key = channels.reduce((value, n) => (value << 8) + n, 0);
  if (!colorCache.has(key)) {
    const [red, green, blue, alpha] = channels;
    colorCache.set(key, 'rgba(' + red + ',' + green + ',' + blue + ',' + alpha / 255 + ')');
  }
  return colorCache.get(key);
}

function text(ptr, len, x, y, w, h, font, r, g, b, a) {
  const [size, center, css] = fonts.get(font);
  ctx.font = css;
  ctx.fillStyle = color(r, g, b, a);
  ctx.textAlign = center ? "center" : "left";
  ctx.textBaseline = center ? "middle" : "top";
  const string = decoder.decode(memory(ptr, len));
  if (center || ctx.measureText(string).width <= w || h <= size * 1.75) {
    ctx.fillText(string, center ? x + w / 2 : x, center ? y + h / 2 : y, w);
    return;
  }
  let row = "";
  let top = y;
  for (const character of string) {
    if (row && ctx.measureText(row + character).width > w) {
      ctx.fillText(row, x, top, w);
      top += size * 1.35;
      row = "";
      if (top + size > y + h) return;
    }
    row += character;
  }
  if (row) ctx.fillText(row, x, top, w);
}

function bitmap(w, h, ptr) {
  const src = memory(ptr, w * h * 4);
  const image = new ImageData(w, h);
  for (let i = 0; i < src.length; i += 4) {
    const alpha = src[i + 3];
    if (alpha) {
      image.data[i] = src[i + 2] * 255 / alpha;
      image.data[i + 1] = src[i + 1] * 255 / alpha;
      image.data[i + 2] = src[i] * 255 / alpha;
      image.data[i + 3] = alpha;
    }
  }
  const sprite = document.createElement("canvas");
  sprite.width = w;
  sprite.height = h;
  sprite.getContext("2d").putImageData(image, 0, 0);
  return bitmaps.push(sprite) - 1;
}

function fullscreen() {
  const change = document.fullscreenElement ? document.exitFullscreen() : canvas.requestFullscreen?.();
  change?.catch(() => {});
}

const env = {
  canvas_begin(scale, r, g, b) {
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.fillStyle = color(r, g, b);
    ctx.fillRect(0, 0, canvas.width, canvas.height);
  },
  canvas_origin(scale, x, y) {
    ctx.setTransform(scale, 0, 0, scale, x * scale, y * scale);
  },
  canvas_rect(x, y, w, h, stroke, r, g, b, a) {
    if (stroke) {
      ctx.strokeStyle = color(r, g, b, a);
      ctx.lineWidth = stroke;
      ctx.strokeRect(x, y, w, h);
    } else {
      ctx.fillStyle = color(r, g, b, a);
      ctx.fillRect(x, y, w, h);
    }
  },
  canvas_circle(x, y, radius, r, g, b, a) {
    ctx.beginPath();
    ctx.arc(x, y, radius, 0, 2 * Math.PI);
    ctx.fillStyle = color(r, g, b, a);
    ctx.fill();
  },
  canvas_text: text,
  canvas_bitmap: bitmap,
  canvas_sprite(id, x, y, w, h, alpha) {
    ctx.globalAlpha = alpha;
    ctx.drawImage(bitmaps[id], x, y, w, h);
    ctx.globalAlpha = 1;
  },
  browser_fullscreen: fullscreen,
  browser_error(ptr, len) {
    console.error(decoder.decode(memory(ptr, len)));
  },
};

function dimensions() {
  const rect = canvas.getBoundingClientRect();
  const ratio = Math.min(window.devicePixelRatio || 1, 3);
  return [Math.max(1, Math.round(rect.width * ratio)), Math.max(1, Math.round(rect.height * ratio))];
}

function resize() {
  const [w, h] = dimensions();
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w;
    canvas.height = h;
    wasm.yy_event(3, w, h);
  }
}

function point(event) {
  const rect = canvas.getBoundingClientRect();
  return [(event.clientX - rect.left) * canvas.width / rect.width,
    (event.clientY - rect.top) * canvas.height / rect.height];
}

function save() {
  if (!wasm.yy_can_autosave()) return true;
  try {
    const len = wasm.yy_save_size();
    if (len) localStorage.setItem(saveKey, decoder.decode(memory(wasm.yy_save_ptr(), len)));
    return true;
  } catch (error) {
    console.warn("迷宮の記録を保存できませんでした", error);
    return false;
  }
}

async function refreshCache() {
  if (refreshing || !window.confirm("進行を保存し、ゲームのキャッシュを消して再読み込みします。続けますか？")) return;
  if (!save()) {
    window.alert("進行を保存できなかったため、再読み込みを中止しました。");
    return;
  }
  refreshing = true;
  try {
    const base = new URL("./", document.baseURI);
    const next = new URL(location.href);
    next.searchParams.set("_yyrefresh", Date.now().toString(36));
    const files = ["index.html", "main.js", "yydungeon.wasm", "manifest.webmanifest",
      "favicon.ico", "icon-192.png", "icon-512.png", "apple-touch-icon.png"]
      .map(file => new URL(file, base));
    const paths = new Set([base, next, ...files].map(url => url.origin + url.pathname));
    try {
      if ("caches" in window) {
        for (const name of await caches.keys()) {
          const cache = await caches.open(name);
          for (const request of await cache.keys()) {
            const url = new URL(request.url);
            if (paths.has(url.origin + url.pathname)) await cache.delete(request);
          }
        }
      }
    } catch (error) {
      console.warn("ゲームのキャッシュを開けませんでした", error);
    }
    await Promise.all([next, ...files].map(async url => {
      const response = await fetch(url, { cache: "reload" });
      if (!response.ok) throw new Error("HTTP " + response.status + " " + url.pathname);
      await response.arrayBuffer();
    }));
    location.replace(next.href);
  } catch (error) {
    refreshing = false;
    console.warn("再読み込みを中止しました", error);
    window.alert("更新ファイルを取得できませんでした。通信状態を確認して、もう一度お試しください。");
  }
}

async function main() {
  const page = new URL(location.href);
  const refreshToken = page.searchParams.get("_yyrefresh");
  const wasmUrl = new URL("yydungeon.wasm", document.baseURI);
  if (refreshToken) wasmUrl.searchParams.set("_yyrefresh", refreshToken);
  const response = await fetch(wasmUrl);
  if (!response.ok) throw new Error("Wasm HTTP " + response.status);
  wasm = (await WebAssembly.instantiate(await response.arrayBuffer(), { env })).instance.exports;
  try {
    const saved = localStorage.getItem(saveKey);
    if (saved) {
      const bytes = encoder.encode(saved);
      const ptr = wasm.yy_load_buffer(bytes.length);
      if (ptr) memory(ptr, bytes.length).set(bytes);
    }
  } catch (error) {
    console.warn("迷宮の記録を読み込めませんでした", error);
  }
  const [w, h] = dimensions();
  canvas.width = w;
  canvas.height = h;
  wasm.yy_init(w, h);
  status.hidden = true;
  if (refreshToken) {
    page.searchParams.delete("_yyrefresh");
    history.replaceState(history.state, "", page.href);
  }
  new ResizeObserver(resize).observe(canvas);
  window.addEventListener("resize", resize);
  canvas.addEventListener("pointermove", event => {
    if (!refreshing) wasm.yy_event(0, ...point(event));
  });
  canvas.addEventListener("pointerdown", event => {
    if (refreshing || event.button !== 0) return;
    canvas.focus({ preventScroll: true });
    wasm.yy_event(1, ...point(event));
  });
  window.addEventListener("keydown", event => {
    if (refreshing || portraitTouch.matches || event.repeat || event.altKey || event.ctrlKey || event.metaKey) return;
    if ([13, 27, 32, 37, 39, 49, 50, 51, 52, 53, 54, 55, 112, 122].includes(event.keyCode)) {
      event.preventDefault();
      if (event.keyCode === 122) fullscreen();
      else wasm.yy_event(2, event.keyCode, 0);
    }
  });
  window.addEventListener("pagehide", save);
  document.addEventListener("visibilitychange", () => { if (document.hidden) save(); });
  let last = performance.now();
  let saved = last;
  function frame(now) {
    const active = !portraitTouch.matches && !document.hidden && document.hasFocus();
    if (!refreshing && wasm.yy_frame(Math.min(0.25, (now - last) / 1000), Number(active))) {
      void refreshCache();
    }
    last = now;
    if (now - saved >= 2000) {
      save();
      saved = now;
    }
    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}

main().catch(error => {
  console.error(error);
  status.textContent = "迷宮を起動できませんでした。HTTP サーバーからページを開き直してください。";
});
