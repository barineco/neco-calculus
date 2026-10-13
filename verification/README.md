# 微積分演算の検証

このディレクトリの手順では、`neco-calculus` の公開 API が返した値を、Lean で再計算した結果と照合して確かめることができます。

観測値の書き出しには Rust の生成器を使い、判定には Lean の検査器を使います。検査器は、各観測の入力から値を再計算して観測値と照合し、演算族ごとの条件が成り立つかを確かめます。演算族は、検査の対象とする公開 API の操作の種類で、観測の `operation` の値で指定します。

検査器が `verified` と判定した観測は、演算族ごとに Lean で定義した仕様を満たします。例えば多項式の厳密評価の仕様は、観測値が点における多項式の値と等しいことです。

この性質は、`lean/NecoCalculusVerify/Soundness.lean` の健全性定理で証明されています。観測バッチと判定の形式については、`protocol/README.md` を参照してください。

## 必要なもの

- Lean: `lean/lean-toolchain` が指す版
- Rust: stable のツールチェーン
- `git`
- `jq`: `scripts/run-smoke.sh` が生成要求と観測器の出力を読むために使う
- `neco-lean/neco-lean-main`: Lean の補助ライブラリ
- `neco-algebra/neco-algebra-main`: 検査器が使う代数の Lean ライブラリと、生成器が使う Rust のクレート
- `mathlib`: Lake が取得する

## リポジトリの配置

依存するリポジトリと本リポジトリは、共通のディレクトリ `<root>` の下へ、それぞれ `git clone` で複製します。

- `neco-lean/neco-lean-main`
- `neco-algebra/neco-algebra-main`
- `neco-calculus/neco-calculus-main`

各リポジトリの main ブランチの作業ツリーは、`<repo>/<repo>-main` に置きます。`mathlib` の配置は Lake が行います。
検査器と生成器は、依存するリポジトリを、それぞれの設定ファイルに書いた相対パスで参照し、この相対パスは上の配置を前提としています。

- `lean/lakefile.toml`: 検査器の設定。neco-algebra の Lean ライブラリへの相対パスを持つ
- `rust/crates/calculus-observation-producer/Cargo.toml`: 生成器の設定。neco-algebra のクレートへの相対パスを持つ

## Lean の検査器

`lean` ディレクトリで、mathlib の構築済みの成果物を取得してから検査器を構築します。

```sh
cd neco-calculus/neco-calculus-main/verification/lean
lake exe cache get
lake build
```

構築した検査器は観測バッチを読み取り、検査結果を標準出力へ書き出します。観測バッチの代わりに `-` を渡すと、標準入力から読み取ります。

```sh
.lake/build/bin/neco-calculus-checker check <観測バッチ>
```

## Rust の生成器

`rust` ディレクトリで生成器を構築し、実行します。

```sh
cd neco-calculus/neco-calculus-main/verification/rust
cargo build --locked
cargo run --locked -p calculus-observation-producer -- describe
cargo run --locked -p calculus-observation-producer -- generate --request <request>
cargo run --locked -p calculus-observation-producer -- self-check --request <request> --batch <観測バッチ>
```

`<request>` には、正規 JSON で書いた生成要求のファイルを指定します。`-` を渡すと、生成要求を標準入力から読み取ります。各コマンドは、結果を正規 JSON で標準出力へ書き出します。

- `describe`: 生成器の版、演算族ごとの入力の生成規則、1 バッチの観測件数の上限 ( 2,000 件 ) 、構築に使った Rust コンパイラの識別子
  - 入力の生成規則: 演算族ごとに宣言した、検査する条件、要求する観測の種類、失敗観測が持つ型付きの失敗の種類、入力を区分する軸、入力値の選び方
  - 入力を区分する軸: 入力値の符号や区間の幅のように、生成する入力を分ける観点
  - 入力値の選び方: 生成要求に収めた乱数の値 ( `selectors` ) から、軸ごとの区分と具体的な入力値を決める規則
- `generate`: 生成要求に従って公開 API を呼び出した結果を、観測値として収めた観測バッチ
- `self-check`: 渡した観測バッチが、同じ生成要求で公開 API を呼び出した結果と一致するかを確かめた記録

## 観測器

`observer` ディレクトリの観測器は、保存した観測バッチと検査器の結果を照合し、判定ごとの件数を出力します。照合では、検査結果に記録されたダイジェストと観測識別子の並びが、観測バッチのものと一致することを確かめます。

```sh
cd neco-calculus/neco-calculus-main/verification/observer
cargo run --locked -- --batch <観測バッチ> --checker-result <検査器の結果>
```

## 補助のスクリプト

- `scripts/check-canonical.sh <json-file-or-directory>...`: JSON が正規形であることを確認
- `scripts/run-smoke.sh --request <request> --minimum <件数>`: Lean の構築から観測器の照合までを通して実行

`run-smoke.sh` を実行する際は、環境変数 `NECO_VERIFICATION_BOUND_PRODUCER` に生成器の実行ファイルのパスを指定します。

`--minimum` には、1,000 以上で、生成要求が含む入力の件数以下の値を指定します。
全ての観測が `verified` と判定されると、スクリプトは成功します。
書き出された成果物の一覧は、`artifacts/README.md` にあります。
