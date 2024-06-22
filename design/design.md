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

### 暗号化・復号化

* 暗号化方式: AES-256
  * 共通鍵暗号
  * 128 bit (16 byte) ブロック暗号
* 暗号化モード: GCM
  * 認証付き (改ざん検知がついているので自動的にファイル破損検出も可能)

暗号化時のパラメータ

* 暗号鍵 (復号鍵と共通)
  * 鍵長: 256 bit (32 byte)
* nonce
  * 96 bit (12 byte)
  * 毎回新規に生成しなければならない。
  * IV (Initialize Vector) の一部に使っているっぼい。
    確かに nonce という名前の方が毎回生成しないといけないっぽさがあってよいかも。
* AAD
  * Additional Authenticated Data
  * 0 byte 以上の任意長のデータ
  * 平文の追加情報データに対して、暗号・復号とは別に改ざん検知のみ行う機能らしい。
* input
  * 任意長
  * 暗号化する平文データ

暗号化時の結果

* output
  * 長さは input と同じ
* tag
  * 認証データ (改ざん・破損検知に使用)
  * 128 bit (16 byte)
  * output と共に保存する。

### 参考にしたデータ

CRYPTREC暗号リスト (電子政府推奨暗号リスト)

<https://www.cryptrec.go.jp/list.html>

* 共通鍵暗号 AES
  * 秘密鍵 + 公開鍵に分けるメリットがほとんど見当たらないので、共通鍵暗号のうち
  もっともメジャーなものを選択する。
* 鍵長 256
  * 暗号強度要件（アルゴリズム及び鍵長選択）に関する設定基準 2022 年 3 月（Ver. 1.0）
    と、Rust のサンプルが 256 bit になっていたので無難と思われることによる。

![Key Length](keylen.png)
