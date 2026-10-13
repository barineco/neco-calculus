# 微積分演算における検証ベクター

このディレクトリの検証ベクターを生成器と検査器に渡すと、成功観測、失敗観測、改変した観測、形式に反する入力のそれぞれについて、検証の結果を再現できます。

## 観測バッチ

- `known/`: 初版の演算族の成功観測と失敗観測
- `tampered/`: 正規 JSON の形式を保ったまま、`known/` の観測の観測値または入力を改変した観測バッチ
- `invalid/`: 正規 JSON の規約か観測形式に反する入力
- `invalid/non-utf8.hex`: UTF-8 として復号できないバイト列の小文字の十六進表現

10 種類の演算族それぞれの成功観測と失敗観測は、`known/families.json` に収めています。

`known/` と `tampered/` の観測バッチは、形式識別子と観測一覧の項目を持つ正規 JSON 文書です。

- 形式識別子: `schema`
- 観測一覧: `cases`

観測一覧の各観測が持つ項目は、観測識別子、演算族、入力、観測値に限られます。

- 観測識別子: `caseId`
- 演算族: `operation`
- 入力: `input`
- 観測値: `observation`

## 生成要求

`requests/` には、生成器に渡す生成要求を収めています。

- `requests/canonical-request-batch.json`: 10 種類の演算族について、成功と型付き失敗を各 1 件、計 20 件
- `requests/calculus-resource-base-request.json`: `calculus-smoke-request-batch.json` の前半 576 件と同じ入力を収めた生成要求
- `requests/calculus-smoke-request-batch.json`: 10 種類の演算族の観測を繰り返す大量の入力、1,152 件
