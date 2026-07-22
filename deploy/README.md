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
ICTHUB_BIND=127.0.0.1:8003
DATABASE_URL=sqlite://data/icthub.db?mode=rwc
FEIYUE_AUTH_URL=http://127.0.0.1:8001
RUST_LOG=icthub_server=info,tower_http=info
```

## 2. systemd 与 Nginx

需要管理员权限：

```bash
sudo install -m 0644 deploy/icthub-backend.service /etc/systemd/system/icthub-backend.service
sudo install -m 0644 deploy/nginx-icthub-http.conf /etc/nginx/sites-available/icthub
sudo ln -sfn /etc/nginx/sites-available/icthub /etc/nginx/sites-enabled/icthub
sudo systemctl daemon-reload
sudo systemctl enable --now icthub-backend.service
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
curl --noproxy '*' http://127.0.0.1:8003/api/health
curl --noproxy '*' -H 'Host: icthub.top' http://127.0.0.1/api/v1/projects
```

ICTHub 不保存飞跃密码或本地账号。浏览器的 `/auth/*` 请求由 Nginx 转发到飞跃后端，ICTHub 写接口把 Bearer Token 转发给飞跃 `/auth/me` 校验。
