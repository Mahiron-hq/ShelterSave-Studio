# Installing the ShelterSave-Studio website

For the server owner: Debian (current stable release), x64. The website is one executable, `shelter-site`, with the site files built in. It runs behind nginx and listens only on a local address.

## 1. What you need

- A Debian server with root access.
- A subdomain pointing to the server, for example `shelter.mahiron.online`.
- nginx and a TLS certificate, for example from Let's Encrypt with `certbot`.
- The file `shelter-site-1.0.0-linux-x64` from the [GitHub release](https://github.com/Mahiron-hq/ShelterSave-Studio/releases/latest). Compare its SHA-256 checksum with the release page: `sha256sum shelter-site-1.0.0-linux-x64`.

## 2. User and files

```
sudo useradd --system --home /var/lib/shelter-site --shell /usr/sbin/nologin shelter-site
sudo install -m 755 shelter-site-1.0.0-linux-x64 /usr/local/bin/shelter-site
sudo install -d -m 750 -o root -g shelter-site /etc/shelter-site
sudo install -m 640 -o root -g shelter-site config.example.toml /etc/shelter-site/config.toml
```

Open `/etc/shelter-site/config.toml` and check `site_url`; every setting is explained in the file. Mode 640 keeps the bot token readable only by root and the service.

Check the file: `sudo -u shelter-site shelter-site check-config --config /etc/shelter-site/config.toml`.

## 3. Services

```
sudo cp shelter-site.service shelter-watch.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now shelter-site shelter-watch
systemctl status shelter-site shelter-watch
```

- `shelter-site` serves the website and counts visits.
- `shelter-watch` is the watchdog and the Telegram bot. It is a separate process, so it keeps working when the site is down.

systemd creates `/var/lib/shelter-site` by itself. Both services run as the `shelter-site` user with restrictions: no privilege escalation, and writing only to their own folder.

## 4. nginx

1. Copy `nginx.conf` to `/etc/nginx/sites-available/shelter` and set your subdomain and certificate paths.
2. Enable the site: `sudo ln -s /etc/nginx/sites-available/shelter /etc/nginx/sites-enabled/`.
3. Check and apply: `sudo nginx -t && sudo systemctl reload nginx`.

In the sample, the access log writes no IP addresses and no User-Agent strings (format `shelter_anon`), and `/api/hit` is limited in rate and size. The application sends the security headers (CSP, HSTS and others) itself.

Check: `curl -I https://shelter.mahiron.online/en/` returns 200 with `content-security-policy` and `strict-transport-security`.

## 5. Telegram bot

1. Send `/newbot` to [@BotFather](https://t.me/BotFather) and get a token.
2. Find your numeric user id, for example with [@userinfobot](https://t.me/userinfobot).
3. Put `token` and `admin_id` into the `[telegram]` section of the config and restart the watchdog: `sudo systemctl restart shelter-watch`.
4. Send `/help` to the bot.

The bot answers only you, only in a private chat, and ignores everyone else. It uses outgoing connections only: no incoming ports are needed.

Commands:

- `/stats`: short statistics;
- `/status`: site status;
- `/mute 2h`: pause alerts;
- `/unmute`: resume alerts;
- `/help`: list of commands.

## 6. Full statistics

Full statistics are available only on the server:

```
sudo -u shelter-site shelter-site stats --config /etc/shelter-site/config.toml
sudo -u shelter-site shelter-site stats --config /etc/shelter-site/config.toml --from 2026-11-01 --to 2026-11-30 --format csv --out /tmp/stats.csv
```

Formats: `text`, `csv`, `json`. Visitors are unique within one UTC day; monthly and total figures are sums of days.

## 7. Updating

1. Download the new build and check its SHA-256.
2. Keep the current one: `sudo cp /usr/local/bin/shelter-site /usr/local/bin/shelter-site.prev`.
3. Install the new one: `sudo install -m 755 shelter-site-<version>-linux-x64 /usr/local/bin/shelter-site`.
4. Restart: `sudo systemctl restart shelter-site shelter-watch`.

## 8. Rolling back

```
sudo install -m 755 /usr/local/bin/shelter-site.prev /usr/local/bin/shelter-site
sudo systemctl restart shelter-site shelter-watch
```

## 9. Backing up statistics

Statistics are in `/var/lib/shelter-site/stats.redb`, the outage journal is `incidents.log` and the watchdog state is `watch.json`. To copy the database as a whole, stop the site for a few seconds:

```
sudo systemctl stop shelter-site
sudo cp -a /var/lib/shelter-site /root/shelter-site-backup-$(date +%F)
sudo systemctl start shelter-site
```

Without stopping, `stats --format json` exports all counters.

## 10. Removing

```
sudo systemctl disable --now shelter-site shelter-watch
sudo rm /etc/systemd/system/shelter-site.service /etc/systemd/system/shelter-watch.service /usr/local/bin/shelter-site
sudo rm -r /etc/shelter-site /var/lib/shelter-site
sudo userdel shelter-site
```
