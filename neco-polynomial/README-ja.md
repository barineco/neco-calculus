# neco-polynomial

[English](README.md)

`neco-polynomial` は、厳密な実代数的数を係数とする一変数多項式の正規形、形式微分、検証済みの区間評価を提供します。

## 主要機能

**多項式の正規形**

多項式の構成では末尾の零係数を除くため、係数の表現が一意に定まります。どの多項式も、少なくとも 1 個の係数を持ちます。

**厳密評価と区間評価**

厳密評価には Horner 法を使います。区間評価の結果は、指定した二進有理数 ( 分母が 2 の冪である有理数 ) の区間上で、多項式が取る全ての値を含む区間です。

**型付きの失敗**

構成、算術、区間の失敗と、計算資源の上限超過による失敗は、それぞれ別の種類として返します。代数演算の失敗は、その演算を定義したクレートが返した内容を保持しています。

## 仕様と制約

| 操作 | 制約 | 上限に対する結果 |
| --- | --- | --- |
| **構成** | 係数の数が 256 以下 | 上限を超える係数列には `PolynomialFailure::Resource` を返す |
| **微分** | 微分階数 64 以下 | 上限を超える階数には `PolynomialFailure::Resource` を返す |
| **区間評価** | 精度 256 ビット以下 | 上限を超える精度には `PolynomialFailure::Resource` を返す |
| **正規化** | 係数列が空でない | 空の係数列には `PolynomialFailure::EmptyCoefficients` を返す。末尾の零係数を除き、零多項式は 1 個の係数で表す |

## ランタイム構成

既定の構成では、標準ライブラリを使います。

```toml
[dependencies]
neco-polynomial = "0.1"
```

既定の `std` 機能を無効にすると、動的メモリ確保だけを前提に動作するため、標準ライブラリの無い環境でも使えます。多項式と失敗の意味は、どちらの構成でも同じです。

```toml
[dependencies]
neco-polynomial = { version = "0.1", default-features = false }
```

## 依存クレート

* [`neco-algnum`](https://docs.rs/neco-algnum): 実代数的数による厳密な係数と包含区間
* [`neco-bigint`](https://docs.rs/neco-bigint): 厳密な整数と二進有理数の演算

## ライセンス

MIT License です。詳細はリポジトリの [LICENSE](../LICENSE) を参照してください。
