# bkupman

## バックアップサーバ側

[Design Note](design/design.md)

## クライアント側 - Windows

Windows の NTFS にもシンボリックリンクとかジャンクションとか ACL (セキュリティ情報)
とか Alternate Data Stream とか怪しいものが多いので、
コピーと圧縮は WSL ではなく Windows 用のツールを使った方が無難に感じる。
VM 跨ぐとファイルシステムが遅いし。。

### ファイルツリー同期

要は rsync を Windows で行いたいのだが、どれがいいのかは何とも言えない。
Robocopy で我慢できたらそれがいいのかもしれない。

#### Robocopy

Vista あたりから Windows に最初から入っているらしい。
元々 Microsoft 内で海を越えて大量のデータを同期するために開発されたツールらしい。
適当に使うとアクセス制限に引っかかったときに無限リトライしたりする。
頑張って試行錯誤してよさげなコマンドラインパラメータを見つける必要がある。

#### SyncToy

Microsoft 公式が出していたよさげなディレクトリ同期ツールだが、
2021 年頃に消えてしまっている。
その辺に落ちていそうだが、危険な偽物を掴まないよう注意。

#### FastCopy

2024年5月でも積極的に更新が行われている。
シェアウェアで職場環境ではライセンスが必要なので注意。
winget でインストール可能。
コマンドライン版も入る。

### 圧縮

zip は ZIP64 (>2GB) 対応と UTF-8 対応が怪しいレガシーシステムが
蔓延しているようなので回避すること。
同じ環境で展開するため文字コードは特に問題にならないだろうが、
2GB 制限はバックアップ目的には全く役に立たない。

#### 右クリックのコンテキストメニューから

最近は 2GB 問題もクリアして大丈夫そうに見えるが、コマンドラインからの呼び出し方が
分からない部分がダメ。

2024 年、Windows 11 の先端付近では zip, 7z, tar へのサポートがかなり進んでいる
らしい。

#### tar

<https://techcommunity.microsoft.com/t5/containers/tar-and-curl-come-to-windows/ba-p/382409>

実は tar (と curl) が標準で Windows に入るようになっている。
Windows 10 のあるバージョンからだが、入っていないバージョンは既にサポートが切れている。

PowerShell で難しいことをごちゃごちゃやればできたといえばできたが、
本当に求められていたものを今の Microsoft は理解している
(cmd.exe からも使用可能)。

libarchive を使用しており、zip も展開できる(できた)。
ただしヘルプには gzip/bzip2/xz/lzma しか記載がなく、
zip で圧縮する方法があるのかは不明。
別に zip にこだわらなくても tar.gz なりなんなりにすればよい気がしてくる。

```text
> tar --version
bsdtar 3.5.2 - libarchive 3.5.2 zlib/1.2.5.f-ipp

> tar --help
tar.exe(bsdtar): manipulate archive files
First option must be a mode specifier:
  -c Create  -r Add/Replace  -t List  -u Update  -x Extract
Common Options:
  -b #  Use # 512-byte records per I/O block
  -f <filename>  Location of archive (default \\.\tape0)
  -v    Verbose
  -w    Interactive
Create: tar.exe -c [options] [<file> | <dir> | @<archive> | -C <dir> ]
  <file>, <dir>  add these items to archive
  -z, -j, -J, --lzma  Compress archive with gzip/bzip2/xz/lzma
  --format {ustar|pax|cpio|shar}  Select archive format
  --exclude <pattern>  Skip files that match pattern
  -C <dir>  Change to <dir> before processing remaining files
  @<archive>  Add entries from <archive> to output
List: tar.exe -t [options] [<patterns>]
  <patterns>  If specified, list only entries that match
Extract: tar.exe -x [options] [<patterns>]
  <patterns>  If specified, extract only entries that match
  -k    Keep (don't overwrite) existing files
  -m    Don't restore modification times
  -O    Write entries to stdout, don't restore to disk
  -p    Restore permissions (including ACLs, owner, file flags)
bsdtar 3.5.2 - libarchive 3.5.2 zlib/1.2.5.f-ipp
```

#### 7-zip

zip ツールの中では現状多分これが一番いい。
winget でインストール可能。

#### Powershell (非推奨)

Compress-Archive コマンドで圧縮できると見せかけて、
2GB までしか圧縮できないので使い物にならん模様。

dotnet のライブラリを頑張って呼び出せば ZIP64 もいけるっぽいけどよく分からない。

#### Lhaplus (非推奨)

脆弱性が放置されているらしい上に ZIP64 および UTF-8 対応が全然ダメなので、
昔お世話になったことには感謝しつつアンインストールしよう。

#### Python zip

Windows python がインストールされているなら (winget 可能)
実はこれで圧縮展開できる。ZIP64 対応。
パフォーマンスは不明。
WSL 内の python は VM 跨ぎのファイルシステムとなるため非推奨。

```bat
python -m zipfile --help
```

### 自動起動

#### Windows から WSL を呼ぶ

Windows から WSL 内のコマンド呼び出しは `wsl.exe` で簡単にできる。
カレントディレクトリも Windows/Linux のどちらも指定できる。

```bat
> wsl.exe --help
オプション:
   --cd <ディレクトリ>
       指定されたディレクトリを現在の作業ディレクトリとして設定します。
       ~ が使用されている場合、Linux ユーザーのホーム パスが使用されます。パスが
       / の文字で始まる場合、絶対 Linux パスとして解釈されます。
       それ以外の場合、値は絶対 Windows パスである必要があります。

   --distribution, -d <ディストリビューション>
       指定されたディストリビューションを実行します。

   --user, -u <ユーザー名>
       指定されたユーザーとして実行します。

> wsl.exe --cd ~ ls
```

#### タスクスケジューラ

Windows の cron みたいなもの。
多分これを使うのがいい。

#### WSL - cron

使えないことはないけど、WSL2 だと VM が起動している間でないと動かない。。

### サーバへの転送

圧縮ファイルの形になればもはやただの長いバイナリ列で、
ファイルシステムよりもネットワークの方が非常に遅いため、
WSL の rsync を使ってよさそう。

もちろんネットワークマウントして "ファイルツリー同期" と同じツールを使う手もある。

#### rsync

ssh 越しにも使える安心の同期ツール。

#### scp (非推奨)

ssh 越しに圧縮ファイルを1個転送するだけなら scp でよさそうな気もするが、
近年脆弱性が見つかっており、コード全体の設計が古めかしく根本解決が難しそうな雰囲気。
少々オーバースペックなケースであっても rsync 推奨の流れあり。

## クライアント側 - Linux

rsync と tar でええんちゃう…？
