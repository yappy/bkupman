# Backup System

## Requirements

* バックアップを自動的 or 手軽に行えること。
* 暗号化を施し、クラウドストレージと同期できること。
* 要所で破損確認が走ること。
  * できればファイル分割

## Directory Structure

* /
  * repo.toml
    * ルート情報ファイル (兼ロック？)
  * inbox/
    * `prefix_date.tar.xz`
    * `prefix_date.tar.xz.md5sum`
  * repo/
    * `prefix`/
      * `prefix_date.tar.xz` etc.
      * ...
    * `prefix2`/
      * ...
  * crypt/
    * `prefix_date`/
      * `prefix_date.tar.xz.00000`
      * `prefix_date.tar.xz.00001`
      * ...

## 暗号関連

CRYPTREC暗号リスト (電子政府推奨暗号リスト)

<https://www.cryptrec.go.jp/list.html>

* 共通鍵暗号 AES
  * 秘密鍵 + 公開鍵に分けるメリットがほとんど見当たらないので、共通鍵暗号のうち
  もっともメジャーなものを選択する。
* 鍵長 256
  * 暗号強度要件（アルゴリズム及び鍵長選択）に関する設定基準 2022 年 3 月（Ver. 1.0）
    と、Rust のサンプルが 256 bit になっていたので無難と思われることによる。

![Key Length](keylen.png)
