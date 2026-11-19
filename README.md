# ShelterSave-Studio

Редактор сохранений Fallout Shelter для ПК и браузера.
Fallout Shelter save editor for desktop and the web.

[Русский](#русский) · [English](#english)

## Русский

Проект в разработке. Первый выпуск, 1.0.0, ещё не опубликован.

- ПК-версия: Rust и egui, Windows, Linux и macOS, работает без сети.
- Веб-версия: тот же редактор в браузере. Файл сохранения обрабатывается только на устройстве пользователя.

### Чем он отличается от других редакторов

Сведения о других редакторах взяты из их страниц и репозиториев (ноябрь 2026).

- **[Shelter Editor](https://github.com/rakion99/shelter-editor)** (rakion99, браузер). Ресурсы, режим и тема убежища, массовые действия (убрать камни, открыть все комнаты и рецепты, SPECIAL на максимум, вылечить всех), имя, пол, здоровье, уровень, цвет кожи и волос, снаряжение жителей, отряды в пустоши. Отдельные комнаты, задания и питомцы не редактируются.
- **[Wasteland Editor](https://github.com/devvyyxyz/wasteland-editor)** (devvyyxyz, браузер). Ресурсы, жители с внешностью и SPECIAL, склад, список комнат и просмотр планировки, отряды, сезонный пропуск, рецепты, темы, история изменений, проверка файла. По списку планов авторов пока нет отмены правок, редактора заданий, питомцев, детей, происшествий и карты планировки.
- **[FSSE](https://github.com/robot9706/FSSE)** (robot9706, Windows и Android). Репозиторий в архиве с 2020 года, программа не обновляется.
- **[FOSE](https://github.com/goobycorp/fose)** (GoobyCorp). Скрипт командной строки: расшифровывает и зашифровывает файл, править JSON нужно вручную.

Что есть только в ShelterSave-Studio:

- **Карта убежища с перестановкой комнат мышью.** Комнату можно перетащить, объединить, разделить, скопировать, построить новую и убрать камни. Редактор проверяет, что комната помещается и соединена с лифтом, как этого требует игра.
- **Проверка целостности по коду игры.** Находит и исправляет то, из-за чего игра падает или зависает: например, мастерскую не того размера или комнату, которая ещё не открыта.
- **Файл не меняется без нужды.** Сохранение без правок совпадает с исходным побайтно: числа, порядок ключей и неизвестные поля остаются как были.
- **Отмена и повтор любой правки,** в том числе массовой, журнал правок и список всех изменений перед записью.
- **Значения в пределах игры.** Пределы взяты из данных игры, поэтому в обычном режиме нельзя записать то, что игра не примет. Экспертный режим открывает любой ключ файла и дерево JSON.
- **Картинки и внешность из вашей игры.** Предметы, комнаты, темы и житель в полный рост, как в игре. Карточки предметов с описанием, уроном, бонусами, ценой изготовления и продажи. Ни одной картинки из игры в редакторе нет: ПК-версия читает их из установленной игры, сайт — из файла игры, который выбирает сам пользователь.
- **Происшествия:** начать пожар или нашествие в нужной комнате, задать силу, завершить, приостановить случайные происшествия.
- **Задания и цели:** заменить, выполнить, изменить сложность, забыть пройденные задания. Достижения с прогрессом.
- **Дети и семьи:** будущие дети (редкость, близнецы, цвета), беременность, партнёры, предки.
- **Перенос жителя из другого сохранения;** легендарные жители и питомцы из журнала игры.
- **Пустошь:** добыча отряда, темы, стимуляторы, время возвращения, расшифрованный дневник отряда.
- **ПК-версия:** замечает, что игра сохранила файл, и предлагает загрузить новую версию; восстанавливает правки после сбоя; делает резервные копии; не выходит в сеть.
- **Названия как в игре.** Русский и английский интерфейс, названия предметов из официальной русской локализации.
- **Открытый код** под лицензией GPL-3.0.

Сборка из исходников: `cargo run --release -p shelter-save-studio`.

### Правообладатели

ShelterSave-Studio — независимый проект с открытым кодом. Он не связан с ZeniMax Media Inc., Bethesda Softworks LLC, Bethesda Game Studios, Behaviour Interactive Inc. и Microsoft Corporation и не одобрен ими.

Fallout Shelter разработана Bethesda Game Studios при участии Behaviour Interactive, издатель — Bethesda Softworks LLC, компания группы ZeniMax Media (с 2021 года входит в Microsoft). Fallout, Fallout Shelter и связанные логотипы являются товарными знаками или зарегистрированными товарными знаками Bethesda Softworks LLC в США и других странах. Bethesda, Bethesda Softworks, Bethesda Game Studios и ZeniMax являются товарными знаками или зарегистрированными товарными знаками ZeniMax Media Inc. Все названия упоминаются только для обозначения совместимости.

## English

Work in progress. The first release, 1.0.0, is not published yet.

- Desktop: Rust and egui, runs on Windows, Linux and macOS, works offline.
- Web: the same editor in the browser. The save file never leaves the user's device.

### How it differs from other editors

Facts about other editors come from their pages and repositories (November 2026).

- **[Shelter Editor](https://github.com/rakion99/shelter-editor)** (rakion99, browser). Resources, vault mode and theme, bulk actions (remove rocks, unlock all rooms and recipes, max SPECIAL, heal everyone), dweller name, gender, health, level, skin and hair colour, equipment, wasteland teams. Single rooms, quests and pets cannot be edited.
- **[Wasteland Editor](https://github.com/devvyyxyz/wasteland-editor)** (devvyyxyz, browser). Resources, dwellers with looks and SPECIAL, storage, room list and layout view, teams, season pass, recipes, themes, change history, file validation. According to its own roadmap it does not yet have undo, a quest editor, pets, children, incidents or a layout map.
- **[FSSE](https://github.com/robot9706/FSSE)** (robot9706, Windows and Android). The repository has been archived since 2020 and is not updated.
- **[FOSE](https://github.com/goobycorp/fose)** (GoobyCorp). A command-line script that decrypts and encrypts the file; the JSON has to be edited by hand.

Only in ShelterSave-Studio:

- **Vault map with drag-and-drop rooms.** Rooms can be moved, merged, split, copied and built, and rocks removed. The editor checks that a room fits and is connected to an elevator, as the game requires.
- **Integrity check based on the game code.** Finds and fixes what makes the game crash or hang, such as a workshop of the wrong size or a room that is not unlocked yet.
- **No needless changes.** A save written without edits is byte-identical to the original: numbers, key order and unknown fields are kept as they were.
- **Undo and redo for every edit,** bulk edits included, an edit history and a list of all changes before writing.
- **Values within the game's limits.** Limits come from the game data, so normal mode cannot write what the game would reject. Expert mode opens every key of the file and the JSON tree.
- **Pictures and looks from your own game.** Items, rooms, themes and a full-height dweller drawn as in the game. Item cards with description, damage, bonuses, crafting and selling price. The editor contains no game pictures: the desktop version reads them from the installed game, the website from a game file the user picks.
- **Incidents:** start a fire or an attack in a chosen room, set its strength, end it, pause random incidents.
- **Quests and objectives:** replace, complete, change difficulty, forget finished quests. Achievements with progress.
- **Children and families:** future children (rarity, twins, colours), pregnancy, partners, ancestors.
- **Moving a dweller from another save;** legendary dwellers and pets from the game's journal.
- **Wasteland:** team loot, themes, stimpaks, return time, decoded team journal.
- **Desktop version:** notices when the game has saved the file and offers to load it, recovers edits after a crash, makes backups, never goes online.
- **Names as in the game.** Russian and English interface, item names from the official Russian localization.
- **Open source** under GPL-3.0.

Build from source: `cargo run --release -p shelter-save-studio`.

### Rights holders

ShelterSave-Studio is an independent open source project. It is not affiliated with or endorsed by ZeniMax Media Inc., Bethesda Softworks LLC, Bethesda Game Studios, Behaviour Interactive Inc. or Microsoft Corporation.

Fallout Shelter was developed by Bethesda Game Studios with Behaviour Interactive and is published by Bethesda Softworks LLC, a ZeniMax Media company (part of Microsoft since 2021). Fallout, Fallout Shelter and related logos are trademarks or registered trademarks of Bethesda Softworks LLC in the U.S. and/or other countries. Bethesda, Bethesda Softworks, Bethesda Game Studios and ZeniMax are trademarks or registered trademarks of ZeniMax Media Inc. All names are used only to indicate compatibility.

## License

GPL-3.0-or-later, see [LICENSE](LICENSE).

Copyright © 2026 Lev Burmistrov (Mahiron-hq)
