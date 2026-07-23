# ICTHub 部署说明

目标主机：`huawei2`，部署目录：`/home/winbeau/xju-icthub`。

## 1. 用户态构建

```bash
git clone https://github.com/winbeau/xju-icthub.git /home/winbeau/xju-icthub
cd /home/winbeau/xju-icthub
./deploy/build-production.sh
cp backend/.env.example backend/.env
```

生产环境变量至少包含：

```dotenv
ICTHUB_BIND_ADDR=127.0.0.1:8003
ICTHUB_DATABASE_URL=sqlite://data/icthub.db?mode=rwc
FEIYUE_AUTH_URL=http://127.0.0.1:8001
ICTHUB_IMPORT_ROOT=uploads/imports
ICTHUB_IMPORT_MAX_UPLOAD_MB=256
ICTHUB_IMPORT_MAX_UNPACKED_MB=768
ICTHUB_IMPORT_WORKER_POLL_MS=500
ICTHUB_IMPORT_WORKER_LEASE_SECS=120
ICTHUB_FFPROBE_BIN=ffprobe
ICTHUB_FFMPEG_BIN=ffmpeg
ICTHUB_PDFTOPPM_BIN=pdftoppm
ICTHUB_CODEX_ENABLED=false
ICTHUB_CODEX_BIN=tools/codex
ICTHUB_CODEX_HOME=data/codex-home
ICTHUB_CODEX_TIMEOUT_SECS=600
RUST_LOG=icthub_server=info,tower_http=info
```

Codex 已固定在 `vendor/codex` 的版本提交，构建脚本会编译 `codex exec` 并放到
`backend/tools/codex`。首测阶段保持 `ICTHUB_CODEX_ENABLED=false`；本地整理草稿仍然可用。
需要启用真实 Agent 时，如果服务器已有 Codex 原生配置目录，可直接在 `backend/.env` 增加：

```dotenv
ICTHUB_CODEX_ENABLED=true
ICTHUB_CODEX_BASE_URL=https://你的模型代理/v1
ICTHUB_CODEX_MODEL=管理员确认的模型名
ICTHUB_CODEX_HOME=/home/winbeau/codex-config
```

当 `ICTHUB_CODEX_HOME` 中已有 Codex 原生 `auth.json` 时，无需复制或导出 API Key；配置目录
应为 `0700`，`auth.json` 和 `config.toml` 应为 `0600`。也可以改用
`ICTHUB_CODEX_API_KEY_FILE=/etc/icthub/codex-api-key`。凭据不要放入仓库、SQLite 或任务目录。
Codex 的子工具环境使用 `core` 环境白名单并启用默认
`KEY/SECRET/TOKEN` 排除，避免把模型凭据泄露给附件中的命令或脚本。`ICTHUB_CODEX_HOME`
只用于隔离 Codex 运行状态。真实调用前还需要人工确认 Base URL、模型名和配额。

生产环境使用独立 Worker，`icthub-backend.service` 已固定
`ICTHUB_IMPORT_WORKER_EMBEDDED=false`。本地开发不启动独立 Worker 时，可以保留默认值
`true`，由 API 进程内嵌运行单个 Worker。

视频元数据、视频封面和 PDF 首页预览需要：

```bash
sudo apt-get install ffmpeg poppler-utils
```

## 2. systemd 与 Nginx

需要管理员权限：

```bash
sudo install -m 0644 deploy/icthub-backend.service /etc/systemd/system/icthub-backend.service
sudo install -m 0644 deploy/icthub-import-worker.service /etc/systemd/system/icthub-import-worker.service
sudo install -m 0644 deploy/nginx-icthub-http.conf /etc/nginx/sites-available/icthub
sudo ln -sfn /etc/nginx/sites-available/icthub /etc/nginx/sites-enabled/icthub
sudo systemctl daemon-reload
sudo systemctl enable --now icthub-backend.service
sudo systemctl enable --now icthub-import-worker.service
sudo nginx -t
sudo systemctl reload nginx
```

在 DNS 尚未生效时可用 Host 头验证：

```bash
curl --noproxy '*' -H 'Host: icthub.top' http://127.0.0.1/api/health
```

## 3. HTTPS

先将 `icthub.top`（以及需要时的 `www.icthub.top`）A/AAAA 记录指向该服务器。DNS 生效并确认 HTTP 可访问后，使用 webroot 申请证书：

```bash
sudo certbot certonly --webroot \
  -w /home/winbeau/xju-icthub/frontend/dist \
  -d icthub.top -d www.icthub.top
sudo install -m 0644 deploy/nginx-icthub-https.conf /etc/nginx/sites-available/icthub
sudo nginx -t
sudo systemctl reload nginx
```

若不使用 `www` 子域，只为主域申请证书并从 Nginx 的 `server_name` 中删除 `www.icthub.top`。

## 4. 验证

```bash
systemctl --no-pager --full status icthub-backend.service
systemctl --no-pager --full status icthub-import-worker.service
curl --noproxy '*' http://127.0.0.1:8003/api/health
curl --noproxy '*' -H 'Host: icthub.top' http://127.0.0.1/api/v1/projects
```

ICTHub 不保存飞跃密码或本地账号。浏览器的 `/auth/*` 请求由 Nginx 转发到飞跃后端，ICTHub 写接口把 Bearer Token 转发给飞跃 `/auth/me` 校验。
