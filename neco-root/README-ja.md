# neco-root

[English](README.md)

`neco-root` は、厳密多項式について、検証済みの根区間、細分化の検証結果、重複度の分類、局所的な所属解析を提供します。

## 主要機能

**検証済みの根区間**

検証済みの符号変化、または偶数重複度を持つ厳密な二進有理数根が根区間を確立します。公開結果は内部表現を保持し、観測用メソッドから意味を公開します。

**検証結果を保持する細分化**

二分法と Newton 二分法が収束の検証結果を構成します。微分の包含区間が零を含む場合は二分法を選択し、理由と回数を検証結果へ記録します。

**局所的な所属解析**

候補の解析が局所的な根と重複度を確立します。区間評価の範囲が零を除外すると、正または負の符号を確立します。

**型付きの失敗の来歴**

入力、多項式、区間、代数演算、計算資源の失敗は、個別の内容を `RootFailure` に保持します。

## 仕様と制約

| 操作 | 制約 | 上限に対する結果 |
| --- | --- | --- |
| **候補解析** | 候補点 4,096 件以下 | 上限を超える要求へ `RootFailure::ResourceLimit` を返却 |
| **重複度解析** | 微分階数 64 以下 | 上限を超える階数へ `RootFailure::ResourceLimit` を返却 |
| **細分化** | 反復回数 19 以下 | 上限を超える回数へ `RootFailure::ResourceLimit` を返却 |
| **区間評価** | 精度 256 bit 以下 | 上限を超える精度へ `RootFailure::ResourceLimit` を返却 |
| **分類** | 検証結果が各公開判定を確立 | 検証結果が未完了の間は明示的な未確定結果を生成 |

## ランタイム構成

既定では標準ライブラリと連携します。

```toml
[dependencies]
neco-root = "0.1"
```

標準ライブラリのない環境では、動的メモリ確保だけを使います。結果と失敗の意味は同じです。

```toml
[dependencies]
neco-root = { version = "0.1", default-features = false }
```

## 依存クレート

* [`neco-polynomial`](https://docs.rs/neco-polynomial): 正規多項式、微分、区間評価
* [`neco-algnum`](https://docs.rs/neco-algnum): 厳密な代数的値と包含区間
* [`neco-bigint`](https://docs.rs/neco-bigint): 厳密な二進有理数の演算
* [`neco-formsum`](https://docs.rs/neco-formsum) と [`neco-monomial`](https://docs.rs/neco-monomial): 代数的な検証結果の形式

## ライセンス

MIT License (詳細はリポジトリの [LICENSE](../LICENSE) を参照)
