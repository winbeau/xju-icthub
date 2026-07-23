#!/usr/bin/env bash
set -euo pipefail

repo_dir="${1:-/home/winbeau/xju-icthub}"
token_staging="/home/winbeau/.icthub-tunnel.env.tmp"

cd "$repo_dir"
sudo -v

sudo install -d -m 0755 /etc/cloudflared
if [[ -s "$token_staging" ]]; then
    sudo install -m 0600 "$token_staging" /etc/cloudflared/icthub.env
    rm -f "$token_staging"
else
    sudo test -s /etc/cloudflared/icthub.env
fi

sudo install -m 0644 \
    deploy/cloudflared-icthub.service \
    /etc/systemd/system/cloudflared-icthub.service
sudo install -m 0644 \
    deploy/nginx-icthub-tunnel.conf \
    /etc/nginx/sites-available/icthub-tunnel
sudo ln -sfn \
    /etc/nginx/sites-available/icthub-tunnel \
    /etc/nginx/sites-enabled/icthub-tunnel

sudo systemctl daemon-reload
sudo nginx -t
sudo systemctl enable --now cloudflared-icthub.service
sudo systemctl reload nginx

systemctl is-active \
    cloudflared-icthub.service \
    nginx \
    icthub-backend.service \
    icthub-import-worker.service \
    feiyue-backend.service

curl --noproxy '*' -fsS http://127.0.0.1:8482/ >/dev/null
printf 'ICTHub tunnel origin is ready at http://127.0.0.1:8482\n'
