# 迷宮工房 · YYDungeon

ゴブリン工務隊に命令を出して迷路を広げ、宝箱で冒険者をおびき寄せるダンジョン運営ゲームです。

## 起動

Windows 版は Rust と MSVC のビルド環境で起動します。

```powershell
cargo run --locked --release --bin yydungeon-win
```

ブラウザ版は Rust、Node.js、プレビュー用の Python を使います。npm パッケージのインストールは不要です。

```powershell
rustup target add wasm32-unknown-unknown
node tools/build.mjs
python -m http.server 8765 --bind 127.0.0.1 --directory dist
```

[http://127.0.0.1:8765/](http://127.0.0.1:8765/) を開きます。GitHub Actions からも Windows の実行ファイルとブラウザ用の静的ファイルを取得できます。

## 遊び方

1. 右の工務隊で指示を選び、迷路の区画をクリックまたはタップします。宝箱、モンスター、罠は通路に置けます。掘削は通路に接した内壁に使えます。
2. 宝箱を増やすと冒険者の人数が増えます。レア度を選んで新設または既存の宝箱を指定すると、より強い冒険者を呼べます。宝箱を調べると入口から宝箱までの道筋を確認できます。
3. 42秒ごとに襲撃が始まります。「冒険者を呼び込む」で早めることもできます。撃退した冒険者の死体から骨材を集め、持ち物と金貨を回収します。強い冒険者ほど報酬が増えます。玉座まで通すと氷の主が交戦し、HPが0になるとゲームオーバーです。
4. その階で6人、14人を撃退すると迷路と配置上限が広がり、金貨と宝箱のレア度が増えます。規模3で20人撃退し、金貨140・骨材8を集めると新しい階を掘れます。
5. 上層を中ボスに任せると迎撃と回収を自動で行います。表示されたレベル以上の冒険者や迎撃しきれなかった冒険者は下の階へ向かいます。階の切り替えは画面左上の矢印を使います。

**キー操作**　F1 遊び方 / 1〜7 建設指示 / Enter 襲撃開始 / Space 一時停止 / ← → フロア切り替え / Esc 指示・案内を閉じる / F11 全画面。

両版とも進行を自動保存します。Windows は `%APPDATA%\yydungeon\dungeon.json`、ブラウザはサイト単位のローカルストレージを使います。

## 共通ソース

`src/dungeon.rs` が迷路・建設・戦闘・進行・保存形式を、`src/screen.rs` が画面と操作を担当します。同じ Rust を Windows 実行ファイルと WebAssembly にビルドし、Windows は YYHistory 由来の Direct3D11・Direct2D・DirectWrite、Web は Canvas 2D に描画します。画像は両版に同梱します。[画像素材と生成プロンプト](assets/README.md)

```powershell
cargo test --locked --lib
node tools/build.mjs
node tools/wasm-smoke.mjs
```
