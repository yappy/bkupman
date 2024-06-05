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
  * repo/
