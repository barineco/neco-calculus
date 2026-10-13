# 微積分演算における検証成果物の配置

`../scripts/run-smoke.sh` か観測器のテストを実行すると、検証の成果物がこのディレクトリへ書き出され、実行後に観測バッチ、検査結果、公開 API との照合の記録を確かめることができます。

観測器のテストは `../observer` ディレクトリの `cargo test` で実行でき、構築済みの Lean の検査器が必要です。

どちらの実行でも、検査器はここに書き出された観測バッチを読み、観測器はその観測バッチと検査結果を照合します。成果物は実行のたびに生成し直され、Git の追跡対象から除かれています。

- `lean/neco-calculus-checker`: Lean の検査器の実行ファイル ( 判定の形式は `../protocol/README.md` を参照 )
- `lean/calculus-generated-checker-result.json`: Lean の検査器が書き出した検査結果
- `rust/calculus-generated-request.json`: 観測器のテストが生成器へ渡す生成要求 ( `run-smoke.sh` は引数の生成要求を直接読む )
- `rust/calculus-generated-observation-batch.json`: 生成器が公開 API から生成した観測バッチ
- `rust/calculus-generated-public-api-parity.json`: 観測バッチが公開 API の結果と一致するかを生成器が確かめた記録
