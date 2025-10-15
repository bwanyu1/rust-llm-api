# rust-llm-api

Axum 製の API と Next.js 製のフロントエンドで、グループ単位に付箋を共有するアプリです。

## 構成

- `src/` : Axum + SQLx バックエンド（SQLite）
- `public/` : シンプルな静的 UI（バックエンドにバンドル）
- `frontend/` : Next.js 14 アプリ（画面をアカウント / グループ / ボードに分割）

## 起動手順

1. **バックエンド**
   ```bash
   cargo run
   ```
   デフォルトで `http://localhost:8080` で起動し、初回に SQLite スキーマを指定構成でリセットします。

2. **Next.js フロントエンド (`frontend/`)**
   ```bash
   cd frontend
   npm install
   npm run dev
   ```
- 既定では `http://localhost:8080` にリクエストします（Docker でポートを変える場合は `NEXT_PUBLIC_API_BASE_URL` を設定してください。例: `http://localhost:5085`）。
   - `http://localhost:3000` にアクセスすると、以下の 3 画面を行き来できます。
     - `/` : アカウント一覧・新規作成
     - `/groups` : グループ一覧・作成・参加
     - `/board/[groupId]` : グループの付箋ボード

ローカルストレージに選択中のアカウント ID を保存して画面間を連携しています。必要に応じて Git 管理外で `.env.local` を作り、`NEXT_PUBLIC_API_BASE_URL` を設定してください。
