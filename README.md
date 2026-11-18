# ShelterSave-Studio

Редактор сохранений Fallout Shelter для ПК и браузера.
Fallout Shelter save editor for desktop and the web.

[Русский](#русский) · [English](#english)

## Русский

Проект в разработке. Первый выпуск, 1.0.0, ещё не опубликован.

- ПК-версия: Rust и egui, Windows, Linux и macOS, работает без сети.
- Веб-версия: тот же редактор в браузере. Файл сохранения обрабатывается только на устройстве пользователя.

### Чем он лучше других редакторов

Большинство редакторов сохранений Fallout Shelter показывают расшифрованный JSON или несколько полей вроде крышек и ресурсов. ShelterSave-Studio сделан иначе:

- **Правится всё, что есть в файле.** Жители, внешность, питомцы, склад, комнаты на карте убежища, темы, происшествия, задания, пустошь, достижения, рецепты, коллекции, таймеры и события.
- **Понятные названия.** У каждого пункта есть человеческое название, пояснение, единицы и допустимый диапазон. Предметы выбираются из списка с поиском, а не вводятся кодами.
- **Сохранение не испортится.** В обычном режиме значения не выходят за пределы, которые допускает игра. Проверка целостности находит и исправляет ошибки, из-за которых игра падает или зависает.
- **Экспертный режим.** Доступен любой ключ файла: дерево JSON, технические поля и значения вне игровых пределов с предупреждением.
- **Файл не меняется без нужды.** Сохранение без правок совпадает с исходным побайтно: числа, порядок ключей и неизвестные поля остаются как были.
- **Отмена и журнал.** Любую правку, в том числе массовую, можно отменить и повторить. Перед записью видно, что именно изменилось.
- **Защита от потерь.** Резервные копии, атомарная запись, восстановление правок после сбоя. Программа замечает, что игра сохранила файл, и предлагает загрузить новую версию.
- **Картинки из игры.** ПК-версия показывает предметы, комнаты и внешность жителей по файлам игры, установленной у пользователя. В программе картинок нет.
- **Конфиденциальность.** ПК-версия не выходит в сеть. Сайт не получает файл сохранения и не ставит cookie.
- **Русский и английский** интерфейс. Названия игровых объектов совпадают с официальной русской локализацией.
- **Открытый код** под лицензией GPL-3.0.

Сборка из исходников: `cargo run --release -p shelter-save-studio`.

### Правообладатели

ShelterSave-Studio — независимый проект с открытым кодом. Он не связан с ZeniMax Media Inc., Bethesda Softworks LLC, Bethesda Game Studios, Behaviour Interactive Inc. и Microsoft Corporation и не одобрен ими.

Fallout Shelter разработана Bethesda Game Studios при участии Behaviour Interactive, издатель — Bethesda Softworks LLC, компания группы ZeniMax Media (с 2021 года входит в Microsoft). Fallout, Fallout Shelter и связанные логотипы являются товарными знаками или зарегистрированными товарными знаками Bethesda Softworks LLC в США и других странах. Bethesda, Bethesda Softworks, Bethesda Game Studios и ZeniMax являются товарными знаками или зарегистрированными товарными знаками ZeniMax Media Inc. Все названия упоминаются только для обозначения совместимости.

## English

Work in progress. The first release, 1.0.0, is not published yet.

- Desktop: Rust and egui, runs on Windows, Linux and macOS, works offline.
- Web: the same editor in the browser. The save file never leaves the user's device.

### Why it is better than other editors

Most Fallout Shelter save editors show the decrypted JSON or a few fields such as caps and resources. ShelterSave-Studio works differently:

- **Everything in the file can be edited.** Dwellers, looks, pets, storage, rooms on the vault map, themes, incidents, quests, the wasteland, achievements, recipes, collections, timers and events.
- **Plain names.** Every item has a readable name, a short explanation, units and an allowed range. Items are picked from a searchable list instead of typed as codes.
- **Saves stay safe.** In normal mode values stay inside the limits the game accepts. The integrity check finds and fixes problems that make the game crash or hang.
- **Expert mode.** Every key of the file is reachable: the JSON tree, technical fields, and values outside game limits with a warning.
- **No needless changes.** A save written without edits is byte-identical to the original: numbers, key order and unknown fields are kept as they were.
- **Undo and history.** Every edit, including bulk edits, can be undone and redone. Before writing you see exactly what changed.
- **Protection against loss.** Backups, atomic writes, recovery of edits after a crash. The editor notices when the game has saved the file and offers to load the new version.
- **Game pictures.** The desktop version shows items, rooms and dweller looks from the game installed on the user's computer. The editor itself ships no pictures.
- **Privacy.** The desktop version never goes online. The website never receives the save file and sets no cookies.
- **Russian and English** interface. Game object names match the official Russian localization.
- **Open source** under GPL-3.0.

Build from source: `cargo run --release -p shelter-save-studio`.

### Rights holders

ShelterSave-Studio is an independent open source project. It is not affiliated with or endorsed by ZeniMax Media Inc., Bethesda Softworks LLC, Bethesda Game Studios, Behaviour Interactive Inc. or Microsoft Corporation.

Fallout Shelter was developed by Bethesda Game Studios with Behaviour Interactive and is published by Bethesda Softworks LLC, a ZeniMax Media company (part of Microsoft since 2021). Fallout, Fallout Shelter and related logos are trademarks or registered trademarks of Bethesda Softworks LLC in the U.S. and/or other countries. Bethesda, Bethesda Softworks, Bethesda Game Studios and ZeniMax are trademarks or registered trademarks of ZeniMax Media Inc. All names are used only to indicate compatibility.

## License

GPL-3.0-or-later, see [LICENSE](LICENSE).

Copyright © 2026 Lev Burmistrov (Mahiron-hq)
