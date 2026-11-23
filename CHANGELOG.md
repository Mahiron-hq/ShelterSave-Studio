# Журнал изменений / Changelog

Формат: [Keep a Changelog](https://keepachangelog.com/ru/1.1.0/), версии: [SemVer](https://semver.org/lang/ru/).
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), versions: [SemVer](https://semver.org/).

## [1.0.0]

### Добавлено

- ПК-версия для Windows, Linux и macOS (Intel и Apple Silicon): портативный файл и установщик.
- Веб-версия на https://shelter.mahiron.online: тот же редактор в браузере, файл сохранения не покидает устройство.
- Правка всего содержимого сохранения: ресурсы и коробки, склад, комнаты на карте убежища, темы, чертежи и рецепты, жители, внешность, снаряжение, питомцы, Мистер Помощник, семьи и дети, задания и цели, достижения, пустошь, происшествия, события и таймеры. Экспертный режим с деревом JSON.
- Проверка целостности с исправлением ошибок, из-за которых игра падает или зависает.
- Комнаты на карте объединяются с соседними так же, как в игре; комнату можно разделить на части. В экспертном режиме объединение можно отключить.
- Оживление жителей, погибших в убежище, в пустоши и на задании: отряд продолжает путь или возвращается домой.
- Отмена и повтор, журнал правок, список изменений перед записью, побайтно точная запись без правок.
- Картинки и фразы дневника пустошей из игры пользователя; в программе и на сайте нет файлов игры.
- ПК-версия: резервные копии, восстановление правок после сбоя, предложение загрузить файл, когда его сохранила игра.
- Интерфейс на русском и английском.
- Сервер сайта для Debian: раздача сайта, анонимная статистика, сторож и Telegram-бот с меню команд; важные оповещения приходят всегда.
- У каждого раздела сайта свой адрес, например /ru/vault/rooms.

### Added

- Desktop version for Windows, Linux and macOS (Intel and Apple Silicon): portable file and installer.
- Web version at https://shelter.mahiron.online: the same editor in the browser; the save file never leaves the device.
- Editing of everything in the save: resources and boxes, storage, rooms on the vault map, themes, blueprints and recipes, dwellers, looks, equipment, pets, Mr. Handy, families and children, quests and objectives, achievements, the wasteland, incidents, events and timers. Expert mode with the JSON tree.
- Integrity check that fixes problems which make the game crash or hang.
- Rooms on the map merge with their neighbours the way the game does; a room can be split into parts. Expert mode can turn merging off.
- Reviving dwellers who died in the vault, in the wasteland and on a quest: the team goes on or heads home.
- Undo and redo, edit history, list of changes before writing, byte-identical output without edits.
- Pictures and wasteland journal phrases from the user's own game; the program and the website contain no game files.
- Desktop version: backups, recovery of edits after a crash, an offer to load the file when the game has saved it.
- Russian and English interface.
- Website server for Debian: site, anonymous statistics, watchdog and Telegram bot with a command menu; important alerts always arrive.
- Every section of the website has its own address, for example /en/vault/rooms.

[1.0.0]: https://github.com/Mahiron-hq/ShelterSave-Studio/releases/tag/v1.0.0
