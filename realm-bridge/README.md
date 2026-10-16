# realm-bridge — Этап 1

Независимый консольный C#-бинарник: читает `client.realm` lazer и атомарно
применяет JSON-патчи. Используется только для взаимодействия с базой — все
файловые операции (копирование ресурсов, хэширование, бэкап `client.realm`)
делает вызывающая сторона на Rust *до* запуска моста.

## Почему Dynamic API, а не сгенерированные классы

Мост открывает базу с `IsDynamic = true` и не линкует скомпилированные модели
из исходников игры. Схема считывается прямо из файла `client.realm`, поля
адресуются по имени через `dynamic`. Это осознанный компромисс: мост переживёт
обновления схемы lazer (новые поля, новый `SchemaVersion`) без пересборки —
пока не переименуют сами классы `Skin`/`File` или их ключевые поля.

## Сборка

```bash
cd realm-bridge
dotnet restore
dotnet build -c Release

# Для распространения без установленного .NET у пользователя (self-contained + AOT):
dotnet publish -c Release -r linux-x64 --self-contained true -p:PublishAot=true
dotnet publish -c Release -r win-x64   --self-contained true -p:PublishAot=true
```

`PublishAot` требует нативных тулчейнов на машине сборки (на Linux — `clang`,
на Windows — VS Build Tools). Если это проблема на этапе разработки —
временно собирайте без `-p:PublishAot=true` (обычный self-contained), AOT
подключите ближе к релизу.

## Контракт CLI

Ровно один JSON-объект в stdout на вызов, вне зависимости от исхода.
Стектрейсы и прочий шум — только в stderr. Код возврата: `0` при
`"success": true`, `1` при `"success": false`.

### `dump-skins --db <path>`

```json
{
  "success": true,
  "data": [
    {
      "id": "2991cfd8-2140-469a-bcb9-2ec23fbce4ad",
      "name": "osu! \"triangles\" (2022)",
      "creator": "team osu!",
      "protected": true,
      "instantiationInfo": "osu.Game.Skinning.TrianglesSkin, osu.Game",
      "hash": "",
      "deletePending": false,
      "files": [
        { "filename": "cursor.png", "fileHash": "3a7f...sha256" }
      ]
    }
  ]
}
```

### `apply-patch --db <path> --patch <patch.json>`

Формат `patch.json` (список операций, применяются в одной write-транзакции —
всё или ничего):

```json
{
  "operations": [
    { "op": "duplicate_skin", "sourceId": "GUID-донора", "newId": "новый-GUID",
      "name": "My mixed skin", "creator": "me" },
    { "op": "add_file", "skinId": "новый-GUID",
      "filename": "cursor.png", "fileHash": "sha256-hex" },
    { "op": "remove_file", "skinId": "новый-GUID", "filename": "cursor-trail.png" },
    { "op": "soft_delete_skin", "id": "GUID-скина" }
  ]
}
```

Замечания:

- `duplicate_skin` копирует список файлов донора (каждая ссылка — новый
  embedded `RealmNamedFileUsage`, указывающий на тот же дедуплицированный
  `File` по хэшу). Новый скин всегда создаётся с `Protected = false`.
- `add_file` не создаёт файл на диске — только регистрирует ссылку в базе.
  Физическое копирование в `files/x/xy/<hash>` обязано произойти *до* вызова
  моста (это Этап 2, файловый менеджер на Rust).
- `add_file` на уже существующее `filename` заменяет старую ссылку, а не
  дублирует её.
- `soft_delete_skin` отказывает на `Protected = true` скинах и не трогает
  файлы на диске — только ставит `DeletePending = true`, как это делает сама
  игра.

## Что дальше (не входит в Этап 1)

- Бэкап (`client.realm` → `client.realm.bak`, плюс лок-файлы) — на стороне
  Rust, перед вызовом моста.
- Проверка запущенных процессов `osu!`/lazer — на стороне Rust, перед вызовом
  моста.
- Копирование ресурсов и SHA-256 хэширование — Этап 2.
