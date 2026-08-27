# neco-polynomial

[English](README.md)

`neco-polynomial` は、厳密な実代数的数を係数とする一変数多項式の正規形、形式微分、検証済みの区間評価を提供します。

## 主要機能

**多項式の正規形**

構成時に末尾の零係数を除去し、一意な係数表現を作ります。全ての多項式は 1 個以上の係数を保持します。

**厳密評価と区間評価**

厳密評価には Horner 法を使います。区間評価は、指定した二進有理数区間上の全ての多項式値を含む区間を返します。

**型付きの失敗の来歴**

構成、算術、区間、計算資源の失敗を区別します。代数演算の失敗は定義元のクレートが返した内容を保持します。

## 仕様と制約

| 操作 | 制約 | 上限に対する結果 |
| --- | --- | --- |
| **構成** | 係数数 256 以下 | 上限を超える列へ `PolynomialFailure::Resource` を返却 |
| **微分** | 微分階数 64 以下 | 上限を超える階数へ `PolynomialFailure::Resource` を返却 |
| **区間評価** | 精度 256 bit 以下 | 上限を超える精度へ `PolynomialFailure::Resource` を返却 |
| **正規化** | 係数列が正規かつ空でない値を表現 | 末尾の零を除去し、零多項式を 1 個の係数で表現 |

## ランタイム構成

既定では標準ライブラリと連携します。

```toml
[dependencies]
neco-polynomial = "0.1"
```

標準ライブラリのない環境では、動的メモリ確保だけを使います。多項式と失敗の意味は同じです。

```toml
[dependencies]
neco-polynomial = { version = "0.1", default-features = false }
```

## 依存クレート

* [`neco-algnum`](https://docs.rs/neco-algnum): 厳密な実代数的係数と包含区間
* [`neco-bigint`](https://docs.rs/neco-bigint): 厳密な整数と二進有理数の演算

## ライセンス

MIT License (詳細はリポジトリの [LICENSE](../LICENSE) を参照)
