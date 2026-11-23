# Установка сайта ShelterSave-Studio

Инструкция для владельца сервера: Debian (актуальный стабильный выпуск), x64. Сайт — один исполняемый файл `shelter-site` со встроенными файлами сайта. Он работает за nginx и слушает только локальный адрес.

## 1. Что понадобится

- Сервер с Debian и доступом root.
- Поддомен, направленный на сервер, например `shelter.mahiron.online`.
- nginx и сертификат TLS, например от Let's Encrypt через `certbot`.
- Архив `ShelterSave-Studio-1.0.0-server-linux-x64.tar.gz` из [выпуска на GitHub](https://github.com/Mahiron-hq/ShelterSave-Studio/releases/latest). Сверьте его контрольную сумму SHA-256 с файлом `SHA256SUMS.txt` выпуска: `sha256sum ShelterSave-Studio-1.0.0-server-linux-x64.tar.gz`. В архиве программа, этот файл и образцы настроек.

## 2. Пользователь и файлы

```
tar xzf ShelterSave-Studio-1.0.0-server-linux-x64.tar.gz
cd ShelterSave-Studio-1.0.0-server-linux-x64
sudo useradd --system --home /var/lib/shelter-site --shell /usr/sbin/nologin shelter-site
sudo install -m 755 shelter-site /usr/local/bin/shelter-site
sudo install -d -m 750 -o root -g shelter-site /etc/shelter-site
sudo install -m 640 -o root -g shelter-site config.example.toml /etc/shelter-site/config.toml
```

Откройте `/etc/shelter-site/config.toml` и проверьте `site_url`. Каждый параметр в файле пояснён. Права 640 нужны, чтобы токен бота читали только root и служба.

Проверка файла: `sudo -u shelter-site shelter-site check-config --config /etc/shelter-site/config.toml`.

## 3. Службы

```
sudo cp shelter-site.service shelter-watch.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now shelter-site shelter-watch
systemctl status shelter-site shelter-watch
```

- `shelter-site` раздаёт сайт и считает посещения.
- `shelter-watch` — сторож и Telegram-бот. Это отдельный процесс, поэтому он продолжает работать, если сайт упал.

Папку `/var/lib/shelter-site` systemd создаёт сам. Службы работают от пользователя `shelter-site` с ограничениями: без повышения прав, с записью только в свою папку.

## 4. nginx

1. Скопируйте `nginx.conf` в `/etc/nginx/sites-available/shelter`, укажите поддомен и пути к сертификату.
2. Включите сайт: `sudo ln -s /etc/nginx/sites-available/shelter /etc/nginx/sites-enabled/`.
3. Проверьте и примените: `sudo nginx -t && sudo systemctl reload nginx`.

В образце журнал доступа не пишет IP-адреса и строки User-Agent (формат `shelter_anon`). Запросы к `/api/hit` ограничены по частоте и размеру. Заголовки безопасности (CSP, HSTS и другие) отдаёт само приложение.

Проверка: `curl -I https://shelter.mahiron.online/en/` — ответ 200 и заголовки `content-security-policy`, `strict-transport-security`.

## 5. Telegram-бот

1. Напишите [@BotFather](https://t.me/BotFather) команду `/newbot` и получите токен.
2. Узнайте свой числовой идентификатор, например у [@userinfobot](https://t.me/userinfobot).
3. Впишите `token` и `admin_id` в раздел `[telegram]` конфигурации и перезапустите сторож: `sudo systemctl restart shelter-watch`.
4. Напишите боту `/help`.

Бот отвечает только вам и только в личном чате, сообщения остальных игнорирует. Он работает через исходящие соединения: входящие порты для него не нужны.

Команды:

- `/stats` — краткая статистика;
- `/status` — состояние сайта;
- `/mute 2h` — приостановить оповещения;
- `/unmute` — включить оповещения;
- `/help` — справка по командам.

## 6. Полная статистика

Полная статистика доступна только на сервере:

```
sudo -u shelter-site shelter-site stats --config /etc/shelter-site/config.toml
sudo -u shelter-site shelter-site stats --config /etc/shelter-site/config.toml --from 2026-11-01 --to 2026-11-30 --format csv --out /tmp/stats.csv
```

Форматы: `text`, `csv`, `json`. Посетители считаются уникальными в пределах суток UTC. Месячные и общие числа — суммы суточных.

## 7. Обновление

1. Скачайте новый архив, сверьте его SHA-256 и распакуйте.
2. Сохраните текущую: `sudo cp /usr/local/bin/shelter-site /usr/local/bin/shelter-site.prev`.
3. Установите новую из папки архива: `sudo install -m 755 shelter-site /usr/local/bin/shelter-site`.
4. Перезапустите: `sudo systemctl restart shelter-site shelter-watch`.

## 8. Откат

```
sudo install -m 755 /usr/local/bin/shelter-site.prev /usr/local/bin/shelter-site
sudo systemctl restart shelter-site shelter-watch
```

## 9. Резервная копия статистики

Статистика лежит в `/var/lib/shelter-site/stats.redb`. Журнал сбоев — `incidents.log`, состояние сторожа — `watch.json`. Чтобы скопировать базу целиком, остановите сайт на несколько секунд:

```
sudo systemctl stop shelter-site
sudo cp -a /var/lib/shelter-site /root/shelter-site-backup-$(date +%F)
sudo systemctl start shelter-site
```

Без остановки можно выгрузить все счётчики в JSON командой `stats --format json`.

## 10. Удаление

```
sudo systemctl disable --now shelter-site shelter-watch
sudo rm /etc/systemd/system/shelter-site.service /etc/systemd/system/shelter-watch.service /usr/local/bin/shelter-site
sudo rm -r /etc/shelter-site /var/lib/shelter-site
sudo userdel shelter-site
```
