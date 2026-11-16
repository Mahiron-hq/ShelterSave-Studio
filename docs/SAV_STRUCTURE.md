# Fallout Shelter: структура расшифрованного сохранения (JSON)

Файл описывает содержимое `.sav` после расшифровки (см. `SAV_FORMAT.md`).

## 0. Источник и оговорки

- Структура снята автоматическим обходом реального расшифрованного сохранения
  `saves/smallMaxedVault.json` (appVersion `2.7.0`, versionCount `498`, 25 жителей,
  22 комнаты). Перечислены все пути полей, которые есть в этом файле, с типами.
- Это сохранение уже «прокачанное» (ресурсы 2147483648, уровни 50), поэтому
  примеры чисел не показывают обычные игровые значения.
- Поля, которых нет в этом файле, здесь не описаны. Игра добавляет поля в новых
  версиях, и часть полей появляется только в определённых ситуациях (помечено
  «необязательное»).
- Смысл поля, помеченный «?», выведен из названия и значений, а не проверен.
  Остальные описания подтверждены поведением данных или игровой логикой.
- Названия полей оставлены как в файле, включая опечатки игры
  (`hasRandonWeaponBeenAssigned`, `buildingLenght`, `firesExtinguised`,
  `questCurrentPerfomance`, `equipedOutfit`, `equipedWeapon`, `Equpping`).

## 1. Общие соглашения

| Что | Как хранится |
|-----|--------------|
| Типы | JSON: object, array, string, число (целое или дробное), boolean, null |
| Идентификаторы жителей | число `serializeId`, на него ссылаются комнаты, команды, списки |
| Идентификаторы комнат | число `deserializeID` (уникально в пределах `vault.rooms`) |
| Цвета (`skinColor`, `hairColor`, `outfitColor`) | целое число `0xAARRGGBB`, где AA всегда `FF` (например `4294967295` = `FFFFFFFF`) |
| Время `timeSaveDate`, `timeGameBegin`, `lastWaveTime`, `measuringSince` | .NET ticks: число 100-наносекундных тиков с 0001-01-01 |
| Время `gameTime`, `time`, `startTime`, `endTime` | секунды игрового таймера (дробное число) |
| Даты квестов (`startDate`, `endDate`) | строки формата `M/D/YYYY h:mm:ss AM/PM` |
| «Нет значения» для ссылки | `-1` (партнёр, комната, задача, смерть) |
| Пустой объект `{}` | допустим (`currentState`, `themeByRoomType`, `SeasonShopWarningModal`) |
| Ключи с номерами | в `happinessManager` ключи создаются динамически: `room<ID>`, `dweller<ID>` |
| Ресурсы | единый набор из 15 ключей, см. раздел 4.4 |

## 2. Корневой объект

Порядок ключей в файле (всего 40):

| Ключ | Тип | Назначение |
|------|-----|-----------|
| `timeMgr` | object | Менеджер времени убежища (раздел 3) |
| `localNotificationMgr` | object | Локальные уведомления (раздел 3) |
| `taskMgr` | object | Планировщик задач убежища (раздел 3) |
| `ratingMgr` | object | Рейтинг/оценки убежища (раздел 3) |
| `specialTheme` | object | Темы комнат и праздничные темы (раздел 3) |
| `dwellers` | object | Жители убежища (раздел 4.1) |
| `constructMgr` | object | Счётчик строительства комнат |
| `vault` | object | Комнаты, ресурсы, склад, достижения (раздел 4) |
| `dwellerSpawner` | object | Очередь ожидающих жителей |
| `deviceName` | string | Имя устройства, на котором создано сохранение |
| `tutorialManager` | object | Состояние обучения и подсказок |
| `objectiveMgr` | object | Задачи (цели) убежища |
| `unlockableMgr` | object | Разблокируемые комнаты и функции |
| `survivalW` | object | Коллекция для режима выживания |
| `ShopWindow` | object | Состояние магазина (стартовый набор) |
| `happinessManager` | object | Данные счастья по комнатам и жителям |
| `refugeeSpawner` | object | Появление беженцев |
| `LunchBoxCollectWindow` | null | Окно сбора ланчбоксов (в примере `null`) |
| `SeasonShopWarningModal` | object | Окно предупреждения сезонного магазина (в примере `{}`) |
| `DeathclawManager` | object | Шанс нападения когтей смерти |
| `PromoCodesWindow` | null | Окно промокодов |
| `JunkGiveAwayWindow` | null | Окно раздачи хлама |
| `MysteriousStranger` | object | Таинственный незнакомец |
| `StatsWindow` | object | Статистика убежища |
| `PromotionFlags` | array of string | Флаги акций |
| `appVersion` | string | Версия игры (в значении есть ведущий пробел: `" 2.7.0"`) |
| `BottleAndCappyMgrSerializeKey` | object | Данные Бутылки и Капсюля |
| `completedQuestDataManager` | object | Выполненные квесты, ежедневные и еженедельные |
| `seasonEventData` | object | Данные сезонных событий |
| `dayToDayRewardMgr` | object | Ежедневные награды |
| `seasonId` | string | Идентификатор сезона (в примере пустая строка) |
| `cameraPosition` | array of 3 float | Позиция камеры в убежище |
| `questSetup` | object | Карта текущего/последнего квеста |
| `questDataManager` | object | Результаты текущего/последнего квеста |
| `questDwellers` | object | Жители в квесте |
| `questDwellerSpawner` | object | Жители, ожидающие в квесте |
| `task` | object | Планировщик задач квеста |
| `time` | object | Менеджер времени квеста |
| `viewManager` | object | Текущий вид (комната) |
| `versionCount` | int | Счётчик версий сохранения (растёт при записи игрой) |

## 3. Служебные менеджеры

### 3.1 `timeMgr` (убежище) и `time` (квест)

Один и тот же набор полей:

| Поле | Тип | Назначение |
|------|-----|-----------|
| `gameTime` | float | Игровое время в секундах |
| `questTime` | int | Время, связанное с квестами (в `time` равно 0) |
| `time` | float | Общий таймер |
| `timeSaveDate` | int | Момент сохранения, .NET ticks |
| `timeGameBegin` | int | Момент начала игры, .NET ticks |

### 3.2 `localNotificationMgr`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `UniqueIDS` | array | Идентификаторы запланированных уведомлений (в примере пусто) |

### 3.3 `taskMgr` (убежище) и `task` (квест)

| Поле | Тип | Назначение |
|------|-----|-----------|
| `id` | int | Последний выданный номер задачи |
| `time` | float | Текущее время планировщика |
| `tasks` | array | Активные задачи |
| `pausedTasks` | array | Приостановленные задачи (пусто в примере) |

Элемент `tasks[]`:

| Поле | Тип | Назначение |
|------|-----|-----------|
| `startTime` | float | Время начала |
| `endTime` | float | Время завершения |
| `id` | int | Номер задачи (на него ссылаются `rushTask`, `taskId`, `taskID`) |
| `paused` | boolean | Приостановлена ли |
| `rescheduleToOldest` | boolean | Переносить ли на самое раннее время? |

### 3.4 `ratingMgr`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `sampleId` | int | Номер последней выборки |
| `dayRatingId` | int | Номер дневного рейтинга |
| `currentSamples` | array | Текущие выборки (пусто) |
| `ratings` | array | Дневные оценки убежища |
| `ratingsLast` | array | Оценки прошлой недели (пусто) |
| `hasWeekRating` | boolean | Есть ли недельная оценка |
| `hasLastWeekRating` | boolean | Есть ли оценка прошлой недели |
| `weekRating` | int | Недельная оценка |
| `lastWeekRating` | int | Оценка прошлой недели |

Элемент `ratings[]`: `rating` (int, оценка), `time` (int, секунды от начала периода), `collected` (boolean, награда получена).

### 3.5 `specialTheme`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `themeByRoomType` | object | Какая тема назначена каждому типу комнаты (в примере `{}`) |
| `eventsThemes` | object | Праздничные темы, 6 фиксированных ключей |
| `lastOverallTheme` | string | Последняя применённая общая тема (в примере `Halloween`) |

Ключи `eventsThemes`: `Cafeteria_Xmas`, `Cafeteria_Halloween`,
`Cafeteria_ThanksGiving`, `LivingQuarters_Xmas`, `LivingQuarters_Halloween`,
`LivingQuarters_ThanksGiving`. Каждый — предмет-тема (формат предмета, раздел 4.3):

| Поле | Тип | Назначение |
|------|-----|-----------|
| `id` | string | Идентификатор темы (совпадает с ключом) |
| `type` | string | Всегда `Theme` |
| `hasBeenAssigned` | boolean | Назначена ли |
| `hasRandonWeaponBeenAssigned` | boolean | Служебный флаг предмета |
| `extraData` | object | Данные темы (ниже) |

`extraData` у темы:

| Поле | Тип | Назначение |
|------|-----|-----------|
| `partsCollectedCount` | int | Собрано частей |
| `IsCraftingInProgress` | boolean | Идёт ли создание |
| `IsCrafted` | boolean | Создана ли |
| `IsClaimed` | boolean | Получена ли |
| `IsClaimedInCraftingRoom` | boolean | Получена в комнате крафта |
| `IsNew` | boolean | Новая (для отметки в интерфейсе) |

### 3.6 `constructMgr`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `roomDeserializeID` | int | Последний выданный `deserializeID` комнаты |

### 3.7 `dwellerSpawner` и `questDwellerSpawner`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `dwellersWaiting` | array | Жители в очереди на появление (пусто в примере) |

### 3.8 `viewManager`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `currentRoom` | int | `deserializeID` открытой комнаты |

## 4. Убежище

### 4.1 `dwellers`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `dwellers` | array | Все жители убежища |
| `actors` | array | Питомцы как отдельные «акторы» (раздел 4.1.3) |
| `id` | int | Последний выданный `serializeId` жителя |
| `mrhId` | int | Последний выданный номер Мистера Помощника |
| `min_happiness` | int | Минимальное счастье (в примере 0) |

#### 4.1.1 Объект жителя (`dwellers.dwellers[]`)

Тот же объект используется в `questDwellers.dwellers[]`; поля, которые есть только
у жителей в квесте, отмечены в 4.1.2.

| Поле | Тип | Назначение |
|------|-----|-----------|
| `serializeId` | int | Уникальный номер жителя |
| `name` | string | Имя (в примере случайные строки из 12 символов, игра подставляет реальное имя сама) |
| `lastName` | string | Фамилия |
| `happiness` | object | `happinessValue` (int, 0–100): счастье |
| `health` | object | Здоровье, см. ниже |
| `deathSource` | int | Причина смерти (0 = нет). Необязательное |
| `experience` | object | Уровень и опыт, см. ниже |
| `relations` | object | Родство и партнёр, см. ниже |
| `gender` | int | `1` = женский, `2` = мужской |
| `stats` | object | SPECIAL, см. ниже |
| `pregnant` | boolean | Беременна |
| `babyReady` | boolean | Ребёнок готов появиться |
| `assigned` | boolean | Назначен на работу в комнате |
| `sawIncident` | boolean | Видел происшествие |
| `WillGoToWasteland` | boolean | Отправится в пустошь |
| `WillBeEvicted` | boolean | Будет выселен |
| `IsEvictedWaitingForFollowers` | boolean | Выселен и ждёт последователей? |
| `skinColor` | int | Цвет кожи, `0xAARRGGBB` |
| `hairColor` | int | Цвет волос, `0xAARRGGBB` |
| `outfitColor` | int | Цвет одежды, `0xAARRGGBB` |
| `pendingExperienceReward` | int | Опыт, ожидающий начисления |
| `hair` | string | Номер причёски (`"01"`…`"22"`). Необязательное |
| `faceMask` | string | Маска лица / тип причёски (`f_hair_01` и т.п.). Необязательное |
| `equipedOutfit` | object | Надетая одежда (формат предмета) |
| `equipedWeapon` | object | Оружие в руках (формат предмета) |
| `equippedPet` | object | Питомец, см. 4.1.4. Необязательное |
| `savedRoom` | int | `deserializeID` комнаты, где житель был (-1 если нет) |
| `wasTemporarilyAssigned` | boolean | Назначен временно. Необязательное |
| `lastChildBorn` | float | Время рождения последнего ребёнка (-1 если не было) |
| `rarity` | string | `Normal`, `Rare`, `Legendary`, `Common` (все встречаются в примере) |
| `deathTime` | int | Время смерти (-1 если жив) |
| `uniqueData` | string | Идентификатор особого (именного) жителя, например `L_Harkness`. Необязательное |
| `isChild` / `child` | boolean | Ребёнок. В примере сохранения этих полей нет ни у одного жителя, имя поля в игре не подтверждено. Необязательное |

**`health`:**

| Поле | Тип | Назначение |
|------|-----|-----------|
| `healthValue` | int | Текущее здоровье |
| `radiationValue` | int | Радиация |
| `permaDeath` | boolean | Окончательная смерть (нельзя воскресить) |
| `lastLevelUpdated` | int | Уровень, для которого последний раз пересчитано здоровье |
| `maxHealth` | int | Максимальное здоровье |

**`experience`:**

| Поле | Тип | Назначение |
|------|-----|-----------|
| `experienceValue` | int | Накопленный опыт |
| `currentLevel` | int | Уровень (1–50) |
| `storage` | int | Опыт в хранилище |
| `accum` | int | Накопленный, но не зачисленный опыт |
| `needLvUp` | boolean | Ждёт повышения уровня |
| `wastelandExperience` | int | Опыт, полученный в пустоши |

**`relations`:**

| Поле | Тип | Назначение |
|------|-----|-----------|
| `relations` | array | Связи (пусто в примере) |
| `partner` | int | `serializeId` партнёра (-1 нет) |
| `lastPartner` | int | `serializeId` прошлого партнёра (-1 нет) |
| `ascendants` | array of 6 int | Предки: `serializeId`; отрицательные значения (до -20) для жителей без записи |

**`stats.stats`** — массив из 8 объектов `{value, mod, exp}`:

| Индекс | Характеристика |
|--------|----------------|
| 0 | Не используется как SPECIAL (служебная ячейка) |
| 1 | S, Сила |
| 2 | P, Восприятие |
| 3 | E, Выносливость |
| 4 | C, Харизма |
| 5 | I, Интеллект |
| 6 | A, Ловкость |
| 7 | L, Удача |

Индексы 1, 2 и 7 подтверждены кодом редактора (S, P, L). Порядок 3–6 соответствует
стандартной последовательности S-P-E-C-I-A-L.

| Поле | Тип | Назначение |
|------|-----|-----------|
| `value` | int | Базовое значение (1–10 в игре) |
| `mod` | int | Бонус от одежды/эффектов? (в примере 0–4) |
| `exp` | int/float | Опыт тренировки этой характеристики (до 918984.6 в примере) |

#### 4.1.2 Поля, которые есть только у жителей в квесте (`questDwellers.dwellers[]`)

| Поле | Тип | Назначение |
|------|-----|-----------|
| `dwellerPosition` | array of 3 float | Позиция в сцене квеста |
| `dwellerRoom` | int | Комната квеста, где находится |
| `dwellerCriticalMeter` | float | Шкала критического удара |
| `dwellerCriticalFactor` | float | Коэффициент критического удара |

`questDwellers` также содержит `actors`, `id`, `mrhId`, `min_happiness` (те же поля, что в `dwellers`).

#### 4.1.3 Актор-питомец (`dwellers.actors[]`, `questDwellers.actors[]`)

| Поле | Тип | Назначение |
|------|-----|-----------|
| `characterType` | int | Тип персонажа (у питомца `3`) |
| `actorDataId` | string | Идентификатор питомца (например `militarymacaw_l`) |
| `serializeId` | int | Номер актора |
| `name` | string | Имя питомца |
| `canCollect` | boolean | Умеет ли собирать |
| `willGoToWasteland` | boolean | Отправится в пустошь |
| `equipment` | object | Инвентарь актора: `storage{resources,bonus}`, `inventory{items}`, `dwellers`, `mrHandyList` (необязательное), `questClues`, `collectedThemes{themeList}` |
| `health` | int | Здоровье (5000 в примере) |
| `death` | boolean | Мёртв |
| `savedRoom` | int | Комната (-1 нет) |
| `FollowedID` | int | `serializeId` жителя, за которым следует питомец |

Формат `equipment.storage` — такой же, как у `vault.storage` (раздел 4.4). В
`questDwellers.actors[].equipment.storage` нет ключей `DummyUltracite` и `PokerChip`.

#### 4.1.4 Питомец жителя (`equippedPet`) и `InitialEquipment[].pet`

Формат предмета (4.3) плюс `extraData`:

| Поле | Тип | Назначение |
|------|-----|-----------|
| `extraData.uniqueName` | string | Имя питомца |
| `extraData.bonus` | string | Тип бонуса (в примере `DamageBoost`) |
| `extraData.bonusValue` | int | Величина бонуса |

### 4.2 `vault`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `rocks` | array | Завалы (скалы) на карте убежища; пусто = нет |
| `ultracite` | array | Данные ультрацита (в примере пусто). Необязательное |
| `rooms` | array | Все комнаты (4.2.1) |
| `storage` | object | Ресурсы убежища (4.4) |
| `inventory` | object | Предметы на складе (4.3) |
| `emergencyData` | object | Чрезвычайные ситуации (4.2.2) |
| `roomConsumption` | object | Задачи потребления энергии: `taskIdOnline`, `taskIDShutDown` |
| `dwellerWaterConsumption` | object | Задача потребления воды: `taskIdOnline` |
| `dwellerFoodConsumption` | object | Задача потребления еды: `taskIdOnline` |
| `lunchboxRandomGenerator` | string | Состояние генератора случайных наград (hex-строка) |
| `LunchBoxesByType` | array of int | Список ланчбоксов, кодирование ниже |
| `LunchBoxDeckNames` | array of string | Названия колод для ланчбоксов (в примере `Normal`). Необязательное |
| `SeasonPassLunchboxClaims` | object | `SeasonPassLunchboxClaims`: array полученных сезонных ланчбоксов. Необязательное |
| `LunchBoxesCount` | int | Общее число ланчбоксов (равно длине `LunchBoxesByType`) |
| `VaultName` | string | Название убежища |
| `VaultMode` | string | `Normal` или `Survival` |
| `VaultTheme` | int | Тема: `0` обычная, `1` Рождество, `2` Хэллоуин, `3` День благодарения |
| `Achievements` | object | Достижения (4.2.3) |
| `wasteland` | object | Состояние пустоши (4.2.4) |

**`LunchBoxesByType`:** каждый элемент — один «ящик», значение означает тип:
`0` ланчбокс, `1` Мистер Помощник, `2` переноска для питомца, `3` стартовый набор.
Количество каждого типа = число элементов с этим значением. `LunchBoxesCount`
должен быть равен длине массива.

#### 4.2.1 Комната (`vault.rooms[]`)

Общие поля:

| Поле | Тип | Назначение |
|------|-----|-----------|
| `emergencyDone` | boolean | Чрезвычайная ситуация в комнате завершена |
| `type` | string | Тип комнаты (ниже) |
| `class` | string | Класс комнаты (ниже) |
| `mergeLevel` | int | Сколько комнат объединено: 1, 2 или 3 |
| `row` | int | Этаж (0 — верхний) |
| `col` | int | Позиция слева по горизонтали (ячейки) |
| `power` | boolean | Есть питание |
| `roomHealth` | object | `damageValue` (float), `initialValue` (float): повреждения |
| `mrHandyList` | array | Мистеры Помощники в комнате |
| `rushTask` | int | Номер задачи ускорения (-1 нет) |
| `level` | int | Уровень улучшения комнаты (1–3) |
| `dwellers` | array of int | `serializeId` жителей в комнате |
| `deadDwellers` | array | Погибшие в комнате |
| `currentStateName` | string | Состояние: `Idle`, `Working` |
| `currentState` | object | Данные состояния (ниже) |
| `deserializeID` | int | Уникальный номер комнаты |
| `assignedDecoration` | string | Назначенное украшение (пустая строка нет) |
| `roomVisibility` | boolean | Видимость |
| `roomOutline` | boolean | Контур |
| `broken` | boolean | Сломана. Необязательное |
| `withHole` | boolean | Есть пролом. Необязательное |

Встречающиеся пары `type`/`class` в примере:
`Entrance/Facility`, `FakeWasteland/Facility`, `Elevator/Utility`,
`Geothermal/Production`, `WaterPlant/Production`, `Cafeteria/Production`,
`LivingQuarters/Facility`, `Storage/Facility`, `Overseer/Quest`, `Radio/Facility`,
`MedBay/Consumable`, `WeaponFactory/Crafting`, `Gym/Training`.

Дополнительные поля зависят от типа комнаты:

| Поле | Тип | У каких комнат | Назначение |
|------|-----|----------------|-----------|
| `currentState.taskId` | int | производственные | Номер текущей задачи |
| `currentState.notificationId` | string | производственные | Идентификатор уведомления (`"-1"` нет) |
| `storage` | object | производственные | `resources` и `bonus` (формат 4.4): накопленная продукция |
| `numberOfProductionCycle` | int | производственные | Число циклов производства |
| `ExperienceRewardIsDirty` | boolean | производственные | Награда опытом ждёт пересчёта |
| `currentState.breedingTaskId` | int | жилая комната | Задача размножения |
| `children` | array | жилая комната | Дети |
| `partners` | array | жилая комната | Пары |
| `newDwellerReady` | boolean | жилая комната | Новый житель готов |
| `onlyIncreaseHappiness` | boolean | жилая комната | Только повышать счастье |
| `IngredientItemIds` | array | комнаты крафта | Идентификаторы ингредиентов |
| `CraftingItemId` | string | комнаты крафта | Предмет в создании |
| `CompletedTime` | int | комнаты крафта | Время готовности |
| `slots` | array | комнаты тренировки | Места тренировки |
| `dwellerWithPendingCompleteTraining` | array | комнаты тренировки | Жители с неполученным результатом |

Элемент `slots[]`: `needLvUp` (boolean), `dwellerID` (int, `serializeId`), `taskID` (int).

#### 4.2.2 `emergencyData`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `active` | boolean | Идёт ли чрезвычайная ситуация |
| `lastWaveTime` | int | Время последней волны, .NET ticks. Необязательное |
| `failedPreviousWave` | boolean | Предыдущая волна провалена. Необязательное |
| `currentWave` | int | Номер текущей волны. Необязательное |
| `graceTimeLeft` | int | Оставшееся время отсрочки (-1 нет). Необязательное |
| `randomEventTaskId` | int | Номер задачи случайного события |

#### 4.2.3 `Achievements`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `objectivesInProgress` | array | Достижения в процессе (4.5) |
| `completed` | array of string | `objectiveID` завершённых достижений |

#### 4.2.4 `wasteland`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `state` | object | `name` (string): состояние, в примере `Running` |
| `teams` | array | Команды в пустоши |
| `cycles` | array | Циклы событий пустоши (ниже) |
| `mrHandyCycles` | array | Циклы Мистеров Помощников |
| `questCycles` | array | Циклы квестов |
| `allTimeTeamsCounter` | int | Счётчик всех отправленных команд |
| `lastSurpriseQuest` | string | Последний неожиданный квест |
| `lastSurprisePopupTime` | float | Время последнего окна неожиданного квеста |

`cycles[]` и `mrHandyCycles[]`: `cycleType` (string: `Radiation`, `Experience`,
`NukaCaps`, `Encounters`, `Item`, `Speech`, `Junk`), `taskId` (int). У `cycles[]`
также необязательные массивы `pendingCycles`, `teamEncounters`, `inProgress`.

### 4.3 Предмет и склад предметов

`vault.inventory.items[]` — каждый предмет отдельная запись (десять одинаковых
предметов = десять записей):

| Поле | Тип | Назначение |
|------|-----|-----------|
| `id` | string | Идентификатор предмета (например `HuntingRifle_Enhanced`, `CombatArmor`, `TeddyBear`) |
| `type` | string | `Weapon`, `Outfit`, `Junk` (также `Pet`, `Theme` в других местах) |
| `hasBeenAssigned` | boolean | Был ли назначен жителю |
| `hasRandonWeaponBeenAssigned` | boolean | Служебный флаг |
| `extraData` | object | Необязательное: данные питомца (4.1.4) или темы (3.5) |

Этот формат используют `equipedOutfit`, `equipedWeapon`, `equippedPet`,
`InitialEquipment[].weapon/outfit/pet`.

### 4.4 Ресурсы (`vault.storage`)

`vault.storage` содержит два объекта с одинаковым набором из 15 ключей:
`resources` (текущие значения) и `bonus` (бонусы, в примере 0).

| Ключ | Значение |
|------|----------|
| `Nuka` | **Крышки** (не Ядер-кола) |
| `Food` | Еда |
| `Energy` | Энергия |
| `Water` | Вода |
| `StimPack` | Стимпаки |
| `RadAway` | Антирадин |
| `Lunchbox` | Ланчбоксы (в примере 999; число реальных ящиков хранится в `LunchBoxesByType`) |
| `MrHandy` | Мистеры Помощники |
| `PetCarrier` | Переноски для питомцев |
| `CraftedOutfit` | Созданная одежда |
| `CraftedWeapon` | Созданное оружие |
| `NukaColaQuantum` | Ядер-кола Квантум |
| `CraftedTheme` | Созданные темы |
| `DummyUltracite` | Ультрацит (служебная запись). Необязательное |
| `PokerChip` | Фишки. Необязательное |

Те же 13–15 ключей встречаются в `StatsWindow.vaultData.collectedRes`,
`previousVResources`, `rooms[].storage` и `actors[].equipment.storage`.

### 4.5 Задачи и достижения (`objectivesInProgress[]`, `objectiveMgr`)

Достижение/задача:

| Поле | Тип | Назначение |
|------|-----|-----------|
| `objectiveID` | string | Идентификатор (например `rush_success_01`, `merge_power_01`) |
| `requirements` | array | Условия |
| `completed` | boolean | Выполнено |
| `incrementLevel` | int | Уровень повышения сложности |

Условие `requirements[]` (поля зависят от типа):

| Поле | Тип | Назначение |
|------|-----|-----------|
| `requirementID` | string | Идентификатор условия (у задач `r1`) |
| `satisfied` | boolean | Выполнено |
| `rushCount` | int | Число ускорений |
| `higherMergeLevelFound` | int | Найден более высокий уровень объединения |
| `currentCreatures` | int | Текущее число существ |
| `currentBabies` | int | Текущее число детей |
| `acceptedRoom` | int | Принятая комната (`deserializeID`) |
| `dweller` | int | `serializeId` жителя |
| `currentInvasions` | int | Число отражённых вторжений |
| `numberOfObjectivesCompleted` | int | Число выполненных задач |
| `currentLegendaryDwellers` | int | Легендарных жителей |
| `currentLegendaryOutfits` | int | Легендарной одежды |
| `currentLegendaryWeapons` | int | Легендарного оружия |
| `nukaCount` | int | Количество крышек |
| `isAchievement` | boolean | Это достижение |
| `currentNumberQuestsCompleted` | int | Выполнено квестов |
| `currentNumberItems` | int | Предметов |
| `currentAllThemeParts` | int | Частей тем |
| `currentNumberQuestDialogChoicesMade` | int | Выборов в диалогах квеста |
| `currentNumberQuestEnemiesToKill` | int | Врагов для убийства |
| `currentNumberLoot` | int | Добычи |
| `currentDwellers` | int | Жителей |
| `currentFires` | int | Потушенных пожаров |

## 5. Остальные разделы

### 5.1 `tutorialManager`

Общие поля:

| Поле | Тип | Назначение |
|------|-----|-----------|
| `phase` | string | Этап обучения (в примере `Phase5`) |
| `taskNumber` | int | Номер задачи обучения |
| `objectivesTutorialMessage` | boolean | Показана подсказка о задачах |
| `lunchboxTutorialMessage` | boolean | Показана подсказка о ланчбоксах |
| `showingObjectiveTutorialMessage` | boolean | Подсказка о задачах на экране |
| `showingLunchboxTutorialMessage` | boolean | Подсказка о ланчбоксах на экране |
| `showWastelandMessageTime` | int | Время показа сообщения о пустоши |
| `showExploreWastelandMessageTime` | int | Время показа сообщения об исследовании |
| `exploreWastelandMessageShown` | boolean | Сообщение показано |
| `skippedTutorial` | int | Обучение пропущено |
| `questTutorialCompleted` | int | Обучение квестам пройдено |
| `canShowSeasonButton` | int | Можно показывать кнопку сезона |
| `seasonTutorialCompleted` | int | Обучение сезонам пройдено |
| `scorchedAttackWarningShown` | int | Предупреждение об атаке Выжженных показано |
| `dwellerSpawnedDuringScorchedAttack` | int | Житель появился во время атаки |
| `scorchedTimerTutorialCompleted` | int | Обучение таймеру Выжженных пройдено |
| `builtUltraciteMine` | int | Построена шахта ультрацита |
| `ultraciteWeaponCraftingTutorialCompleted` | int | Обучение созданию оружия из ультрацита |
| `spinToWinTutorialCompleted` | int | Обучение «Spin to Win» |
| `arrivalTimerTutorialCompleted` | int | Обучение таймеру прибытия |
| `arrivalTimerClaimTutorialCompleted` | int | Обучение получению по таймеру прибытия |
| `canShowArrivalTimerClaimTutorial` | int | Можно показать это обучение |
| `seasonalQuestTutorialCompleted` | int | Обучение сезонным квестам |
| `intialTimerTasks` | array | Начальные задачи таймера |
| `MaleTasksQuant` | int | Число задач для мужчин |
| `FemaleTasksQuant` | int | Число задач для женщин |

Boolean-флаги «контекстной подсказки уже показана» (`true` — подсказка использована):
`ContextualVaultTecObjectives`, `ContextualAddFriends`, `ContextualWasteland`,
`ContextualRadioRoom`, `ContextualWeaponsAndOutfits`, `ContextualTrainDweller`,
`ContextualBabies`, `ContextualDestroyRocks`, `ContextualStorage`,
`ContextualNoRoomForDwellers`, `ContextualUnequipedDweller`,
`ContextualBuildAnElevator`, `ContextualDestroyRockToBuild`,
`ContextualNoBuildZonesAvailableByRock`, `ContextualDestroyRockToAccessNextFloor`,
`ContextualResourcesAlert`, `ContextualIncidentOcurs`, `ContextualLowPowerAlert`,
`ContextualStorageFull`, `ContextualMergeOrUpgradeRoom`,
`ContextualWastelandMessage`, `ContextualObjectivesCompleted`,
`ContextualBabiesTutorial`, `ContextualStimpackMessage`,
`ContextualLunchboxTutorial`, `ContextualRadwayMessage`, `ContextualRoomMerge2`,
`ContextualRoomMerge3`, `ContextualStorage2`, `ContextualEquippingItemsWeapon`,
`ContextualLuck`, `ContextualEquppingItemsPet`, `ContextualCrafting`,
`ContextualDecorations`, `ContextualRequestJunk`, `ContextualJunk`,
`ContextualTriggeredBirth`, `ContextualInventoryFull`,
`ContextualInventoryFullWindow`, `ContextualJunkGiveAway`, `ContextualScrapping`,
`ContextualAssignWith3DTouch`, `ContextualNukaQuantum`,
`ContextualSurpriseQuests`, `ContextualReturningFromQuests`,
`ContextualRadioRoomToggle`, `ContextualCraftTheme`,
`ContextualJoystickNavigationInVault`, `ContextualScorchedAttackWarning`,
`ContextualUltraciteWeapons`, `ContextualBuildUltraciteMine`,
`ContextualBuildUltraciteWeaponFactory`, `ContextualScorchedAttackFail`.

### 5.2 `objectiveMgr` (задачи VaultTec)

| Поле | Тип | Назначение |
|------|-----|-----------|
| `taskID` | int | Номер задачи таймера |
| `canDiscard` | boolean | Можно ли сбросить задачу |
| `nukaQuantumIncrement` | int | Накопление Ядер-колы Квантум |
| `shuffleBags` | array of array of string | Перемешанные очереди `objectiveID` для выдачи (5 массивов) |
| `slotArray` | array | Три активных слота задач |

Элемент `slotArray[]`: `objective` (формат 4.5), `incLevel` (int), `lottery`
(array of boolean, 5 штук, результат розыгрыша награды).

### 5.3 `unlockableMgr`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `objectivesInProgress` | array | Разблокировки в процессе |
| `completed` | array | Выполнено, не получено |
| `claimed` | array of string | Получено (например `StorageUnlock`, `MedbayUnlock`, `OverseerUnlock`) |

### 5.4 `survivalW` (коллекция режима выживания)

| Поле | Тип | Назначение |
|------|-----|-----------|
| `weapons` | array of string | Коды оружия (с префиксом `N`/`O`, например `O24`, `N19`) |
| `outfits` | array of string | Коды одежды |
| `dwellers` | array of string | Коды особых жителей (`OL_Harkness`) |
| `pets` | array of string | Коды питомцев |
| `breeds` | array of string | Коды пород |
| `decorations` | array | Украшения |
| `junk` | array of string | Коды хлама (например `NAlarmClock`) |
| `recipes` | array of string | Открытые рецепты (идентификаторы) |
| `claimedRecipes` | array of string | Полученные рецепты |
| `collectedThemes` | object | `themeList` (array) собранных тем |

### 5.5 `ShopWindow`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `isStarterPackPurchased` | boolean | Стартовый набор куплен |
| `hasStarterPackPopupShown` | boolean | Окно стартового набора показано |

### 5.6 `happinessManager`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `rooms` | array of int | `deserializeID` комнат, у которых есть записи |
| `dwellers` | array of int | `serializeId` жителей, у которых есть записи |
| `room<ID>` | array | Эффекты счастья в комнате с этим ID (динамический ключ) |
| `dweller<ID>` | array | Эффекты счастья у жителя с этим ID (динамический ключ) |

Элемент массива эффектов: `type` (int, тип эффекта: `3` в комнатах, `2` у
жителей), `rh` (int), `in` (boolean), `dc` (int, только в записях комнат; в
примере похож на номер жителя). Расшифровка `rh` и `dc` не подтверждена.

### 5.7 `refugeeSpawner`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `newGameTask` | int | Номер задачи (-1 нет) |
| `succeed` | int | Успешно появившихся (-1 нет) |
| `currentPool` | string | Текущий набор беженцев |
| `collectedAllRefugees` | boolean | Все беженцы получены |

### 5.8 `DeathclawManager`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `deathclawTotalExtraChance` | int | Добавочный шанс нападения |
| `canDeathclawEmergencyOccurs` | boolean | Нападение возможно |
| `deathclawCooldownID` | int | Номер задачи охлаждения (-1 нет) |

### 5.9 `MysteriousStranger`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `currentState` | string | Состояние (в примере `Hiding`) |
| `canAppear` | boolean | Может появиться |
| `timeToAppear` | float | Время до появления |
| `remainingTimeToAppear` | int | Оставшееся время |

### 5.10 `StatsWindow.vaultData` (статистика)

Все поля — int, кроме отмеченных.

| Поле | Назначение |
|------|-----------|
| `collectedRes` | Собранные ресурсы (формат 4.4) |
| `collectedOutfits` / `collectedWeapons` / `collectedDecorations` / `collectedPets` | Собрано предметов по типам |
| `raidersKilled` | Убито рейдеров |
| `elevatorRides` | Поездок на лифте |
| `lunchBoxesOpened` / `petCarriersOpened` | Открыто ланчбоксов / переносок |
| `totalLifetimeDwellers` | Всего жителей за всё время |
| `highestDwellerCount` | Максимум жителей одновременно |
| `babiesBorn` | Рождено детей |
| `dwellersRevived` | Воскрешено |
| `levelsGained` | Получено уровней |
| `specialStatTrained` | Тренировок SPECIAL |
| `takenStimpack` / `takenRadaway` | Использовано стимпаков / антирадина |
| `deadDwellers` | Погибло |
| `evictedDwellers` | Выселено |
| `soldWeapons` / `soldOutfits` / `soldDecorations` / `soldPets` / `soldJunk` | Продано |
| `scrappedOutfits` / `scrappedWeapons` | Разобрано на детали |
| `craftedWeapons` / `craftedOutfits` / `craftedDecorations` / `craftedThemes` | Создано |
| `collectedJunk` | Собрано хлама |
| `successfulRushes` / `failRushes` | Удачных / неудачных ускорений |
| `firesExtinguised` | Потушено пожаров |
| `emergencyStopRaider` / `emergencyStopDeathClaw` / `emergencyStopGhoul` / `emergencyStopRadroach` / `emergencyStopRadscorpion` / `emergencyStopMolerat` / `emergencyStopAlien` | Остановлено нападений по типам |
| `dwellersSentWasteland` | Отправлено в пустошь |
| `wastelandTotalTime` | Общее время в пустоши |
| `wastelandCreaturesKilled` | Убито существ в пустоши |
| `vaultCreatedBeforeUpdate` (boolean) | Убежище создано до обновления |
| `dwellersCustomized` | Изменено внешностей |
| `measuringSince` | Начало измерений, .NET ticks |
| `previousVResources` | Ресурсы на прошлом замере (формат 4.4) |
| `weaponFactoryBuilt` / `outfitFactoryBuilt` (boolean) | Построены фабрики оружия / одежды |

### 5.11 Прочее

| Ключ | Описание |
|------|----------|
| `PromotionFlags` | array of string. Флаги акций (в примере `PetGiveaway`) |
| `appVersion` | string. Версия игры |
| `BottleAndCappyMgrSerializeKey` | `SerializeAccumulatedTriggerChance` (int), `SerializeLocked` (boolean) |
| `seasonId` | string. Идентификатор сезона |
| `cameraPosition` | array of 3 float (x, y, z) |
| `dayToDayRewardMgr` | `states`: array состояний ежедневных наград |
| `seasonEventData` | `currentEvent` (int), `nextEventTime` (string-дата), `claimedEvents` (array), `areAllSeasonEventsClaimed` (boolean), `isNewQuestUnlocked` (boolean) |

### 5.12 `completedQuestDataManager`

| Поле | Тип | Назначение |
|------|-----|-----------|
| `taskID` | int | Номер задачи |
| `completedQuests` | array of string | Выполненные квесты (`TutorialQuest_01`, `MajorQuestline_01_Quest_01`) |
| `foundClues` | array | Найденные улики |
| `standaloneQuestPicker` | object | `currentStandalone` (string): текущий отдельный квест |
| `dailyQuestPicker` | object | `currentDailies[]`, `historyDailies[]` |
| `weeklyQuestPicker` | object | `currentWeeklies[]`, `historyWeeklies[]` |
| `nukaQuantumIncrement` | int | Накопление Ядер-колы Квантум |
| `standaloneQuestSkipper` | object | `skippedQuests` (array) |
| `dailyQuestSkipper` | object | `skippedQuests` (array) |
| `weeklyQuestSkipper` | object | `skippedQuests` (array) |
| `eventQuestCompletedTimes` | object | Времена выполнения событийных квестов (в примере `{}`) |

Элемент ежедневных/еженедельных квестов: `startDate`, `endDate`,
`officialStartDate` (строки-даты), `questName` (string).

### 5.13 `questSetup` (карта квеста)

| Поле | Тип | Назначение |
|------|-----|-----------|
| `buildingLenght` | int | Ширина карты квеста |
| `buildingHeight` | int | Этажи вверх |
| `buildingUnderground` | int | Этажи вниз |
| `seed` | int | Зерно генерации |
| `entry` | int | Точка входа |
| `questConstructionMgr` | object | `roomDeserializeID` (int) |
| `roomList` | array | Комнаты квеста |
| `wasteland` | object | Комната-пустошь (формат комнаты 4.2.1, `type = FakeWasteland`) |
| `entranceId` | int | `deserializeID` входа |

Комната квеста — те же общие поля, что в 4.2.1, плюс (только у комнат со
сценарием):

| Поле | Тип | Назначение |
|------|-----|-----------|
| `questRoomType` | string | Тип квестовой комнаты (`QuestVault`) |
| `roomClearOutResult` | string | Результат зачистки (`Succeeded`) |
| `roomPickUpResult` | string | Результат сбора (`Incompleted`, `Succeeded`) |
| `roomName` | string | Название комнаты |
| `enteringLeft` | boolean | Вход слева |
| `primaryObjective` / `secondaryObjective` / `thirdObjective` | string | Цели (`None`, `ClearOut`) |
| `objectiveTextOverride` | string/null | Ключ замены текста цели |
| `debugRoomId` | string | Отладочный номер (`Room1`…) |
| `xp` | int | Опыт за комнату |
| `mainPath` | boolean | Основной путь |
| `combatParameters` | object | Бой (ниже). Необязательное |
| `Fighters` | array | Бойцы (ниже). Необязательное |

`combatParameters`: `difficulty` (int, -2…2), `enemyType` (string, `Raider`,
`Radroach`), `enemyCount` (int), `boss` (boolean), `loot` (формат добычи ниже),
`dialogue` (string/null).

`Fighters[]`: `fighterHealth` (object `HealthValue`), `Position` (array of 3
float), `IsBoss` (boolean), `NukaLoot` (int), `Death` (boolean).

Формат добычи (`loot`, `QuestLootObtained.Loots[]`):

| Поле | Тип | Назначение |
|------|-----|-----------|
| `LootType` | int | Тип добычи (в примере 0–11; `1` оружие, `2` одежда, `5` крышки, `6` Квантум, `9` особый предмет, `11` хлам; остальные значения не расшифрованы) |
| `LootID` | string | Идентификатор добычи (может быть пустым) |
| `LootQuantity` | int | Количество |
| `FromVaultQuantity` | int | Сколько взято из убежища |
| `InitialEquippedQuantity` | int | Надето в начале |
| `CurrentlyEquippedQuantity` | int | Надето сейчас |

### 5.14 `questDataManager` (результат квеста)

| Поле | Тип | Назначение |
|------|-----|-----------|
| `questDone` | boolean | Квест завершён |
| `cancelled` | boolean | Отменён |
| `questSucceeded` | boolean | Успешно |
| `entranceFlow` | boolean | Вход выполнен |
| `questDifficulty` | int | Сложность |
| `questTeam` | object | Команда (ниже) |
| `accumulatedRetryCost` | int | Накопленная цена повторов |
| `retryCount` | int | Число повторов |
| `incrementedRetryCount` | int | Увеличенный счётчик повторов |
| `vaultCaps` | int | Крышки убежища на момент квеста |
| `vaultQuantum` | int | Квантум убежища на момент квеста |
| `uniqueID` | int | Уникальный номер (совпадает с `questTeam.randomIdentifier`) |

`questTeam`:

| Поле | Тип | Назначение |
|------|-----|-----------|
| `CurrentQuestID` | string | Идентификатор квеста |
| `randomIdentifier` | int | Случайный номер команды |
| `DwellersDictionary` | array of int | `serializeId` жителей команды |
| `QuestLootObtained` | object | `Loots` (array добычи) |
| `InitialEquipment` | array | Снаряжение при входе: `dweller` (int), `weapon`, `outfit`, `pet` (необязательное) |
| `initialStimpakCount` / `initialRadawayCount` | int | Стимпаков / антирадина при входе |
| `currentStimpakCount` / `currentRadawayCount` | int | Сейчас |
| `estimatedDwellerReviveCost` | int | Ориентировочная цена воскрешения |
| `questCurrentPerfomance` | object | Показатели (ниже) |
| `questSpentTime` | float | Затраченное время |

`questCurrentPerfomance`: `numberCombatsWon`, `numberCriticalHitsPerformed`,
`numberPerfectCriticalHitsPerformed`, `numberContainersCollected`,
`numberCapsCollected`, `numberLevelsGained`, `numberLevelsGainedWithBonus` (все int).
