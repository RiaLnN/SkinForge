# osu-mixer-core — Этап 2

Ядро на Rust: резолвер путей lazer, защита от запущенного процесса игры,
файловое хранилище (SHA-256 + content-addressed копирование) и обёртка над
`realm-bridge` (Этап 1) с автоматическим бэкапом базы перед патчем.

**Собрано и протестировано в этой сессии** (`cargo build`, `cargo test` —
11/11 зелёных, плюс ручная проверка CLI). Единственное, что здесь не
проверялось — реальный вызов `realm-bridge`, так как в песочнице нет .NET;
контракт (`BridgeEnvelope`/`SkinDto`/`PatchOperation`) зеркалит JSON из Этапа 1
1-в-1 по именам полей.

## Модули

- **`paths.rs`** — `resolve_lazer_root()` / `resolve_lazer_root_with_override()`.
  Перебирает кандидаты для текущей ОС (`std::env::consts::OS`, без `cfg`-веток,
  так что резолвер типизируется и тестируется на любой ОС):
  - Linux: `$XDG_DATA_HOME/osu` (или `~/.local/share/osu`), плюс Flatpak
    `~/.var/app/sh.ppy.osu/data/osu`.
  - Windows: `%APPDATA%/osu`.
  - macOS: `~/Library/Application Support/osu` (на случай будущей поддержки).

  Также `hashed_storage_path()` — построение пути `files/<h0>/<h0h1>/<hash>`.

- **`process_guard.rs`** — `find_running_game_processes()` /
  `ensure_game_not_running()` через `sysinfo`. Матчит по списку известных
  имён процессов (`osu!`, `osu!.exe`, `osu!lazer`, `osu!lazer.exe`, `osu`)
  **и** по вхождению flatpak app id `sh.ppy.osu` в командную строку — на
  случай, если процесс виден через обёртку (`bwrap`/`flatpak-run`), а не
  напрямую.

- **`storage.rs`** — `sha256_hex()` (потоковое хэширование, 1 MiB буфер) и
  `store_resource()` (копирование с дедупликацией + атомарный rename через
  временный файл, чтобы не оставить частично записанный файл под финальным
  именем).

- **`bridge.rs`** — `RealmBridge` оборачивает вызов `realm-bridge` через
  `std::process::Command`, парсит JSON-конверт ответа. `apply_patch()`
  **всегда** сначала вызывает `ensure_game_not_running()`, затем делает
  таймштампованный бэкап (`client.realm.bak-<YYYYMMDD-HHMMSS>`, плюс
  best-effort копия `.lock`/`.note`/`.management`), и только потом запускает
  мост. `PatchOperation` — конструкторы под каждый тип операции из Этапа 1
  (`duplicate_skin`/`add_file`/`remove_file`/`soft_delete_skin`), чтобы нельзя
  было собрать невалидную комбинацию полей на вызывающей стороне.

## CLI для ручной проверки (`src/bin/cli.rs`)

```bash
cargo run -- find-lazer [--lazer-path <path>]
cargo run -- check-running
cargo run -- hash-file <path> --files-dir <path>
cargo run -- dump-skins --bridge-binary <path/to/realm-bridge> --db <path/to/client.realm>
```

Это не финальный UX — просто способ пощупать ядро руками до GUI на Этапе 4.

## Известные компромиссы / на заметку

- `sysinfo` зафиксирован на `0.30.13` с `default-features = false` (без
  `rayon`) — в этой песочнице новый `rayon-core` требует rustc ≥ 1.80, а
  здесь доступен только 1.75 из apt. На вашей машине с актуальным rustc можно
  спокойно обновить до `sysinfo = "0.31"` и вернуть параллельный скан
  процессов, если список процессов станет большим фактором задержки — для
  однократной проверки перед патчем разница некритична.
- `clap` зафиксирован на `=4.4.18` по той же причине (новее требует
  `edition2024`).
- Бэкап `.lock`/`.note`/`.management` — best-effort (`Result` игнорируется):
  их отсутствие не должно блокировать бэкап, если основной `.realm` файл уже
  скопирован. Если для вас важно бэкапить строго всё или ничего — дайте
  знать, поменяем на строгий вариант.

## Дальше — Этап 3

Парсинг `.osk`/`skin.ini`, категоризация ресурсов, генерация патча с расчётом
SHA-256 для каждого заменяемого спрайта — уже поверх готового `storage.rs` и
`bridge.rs`.

---

## Этап 3 — парсинг и микширование (добавлено, собрано и протестировано)

Новые модули поверх Этапа 2, полный конвейер `.osk → патч` прогнан руками
через CLI на фейковом архиве (распаковка → категоризация → хэширование →
дедупликация → JSON патча) — см. секцию "Ручная проверка" ниже.

- **`skin_ini.rs`** — `SkinIni::parse()`/`read_from_file()`. Читает файл как
  байты и лосси-конвертирует в UTF-8 (многие старые `skin.ini` не в чистом
  UTF-8), понимает BOM, `;`/`//`-комментарии, формат `Key: Value` внутри
  `[Section]`. Типизированные хелперы: `skin_name()`, `skin_author()`,
  `combo_colours()` (только ключи `ComboN`, отсортированы по индексу, прочие
  ключи `[Colours]` вроде `SliderBorder` не путаются с комбо-цветами и
  доступны через `colour_key()`), `fonts()`.

- **`osk.rs`** — `extract_osk()` распаковывает `.osk` (обычный zip) во
  временную/указанную директорию. Защита от zip-slip: записи с `..`/абсолютными
  путями внутри архива пропускаются (`enclosed_name()` возвращает `None` для
  них), а не разворачиваются как есть.

- **`categorize.rs`** — `classify(filename) -> ComponentInfo { category,
  component_key }`. Категории: `Gameplay`/`Cursor`/`Audio`/`Interface`/`Other`.
  Умеет: снимать `@2x`, снимать sampleset-префиксы у звуков
  (`normal-`/`soft-`/`drum-`), снимать цифровой суффикс вариации/кадра
  (`sliderb0` → `sliderb`, `hitnormal2` → `hitnormal`), но **сохранять**
  цифру там, где она значима (`default-3`, `score-7`). Неопознанный `.wav`/
  `.mp3`/`.ogg` всё равно попадает в `Audio` по расширению; совсем неизвестные
  файлы — в `Other`, а не отбрасываются.

- **`mixer.rs`** — склейка всего вместе:
  - `load_donor(root)` — ищет `skin.ini` (сначала в корне, потом на 1-3 уровня
    вглубь — частый случай архива с одним вложенным подкаталогом), парсит его
    и категоризирует все остальные файлы через `walkdir`.
  - `DonorSkin::components_by_category()` / `grouped_by_category()` — то, чем
    будет питаться селектор GUI на Этапе 4.
  - `MixPlan` + `build_patch()` — генерирует `Uuid` нового скина, операцию
    `duplicate_skin` от `base_skin_id`, и по одному `add_file` на каждую
    `ReplacementSelection` (хэширует и кладёт файл в `files_dir` через уже
    существующий `storage::store_resource`).

## Ручная проверка (сделана в этой сессии)

```
Donor.osk (skin.ini + cursor.png + hitcircle.png + sounds/hitnormal.wav + scorebar-bg.png)
  → cargo run -- inspect-donor Donor.osk --extract-to ...
    → верно распознал Name/Author/Combo1-2, разложил 4 файла по 4 категориям
  → cargo run -- build-patch Donor.osk ... --out patch.json
    → патч из 5 операций (1 duplicate_skin + 4 add_file), файлы физически
      легли в files/x/xy/<hash>, повторяющиеся по контенту файлы (в тесте у
      трёх .png было одинаковое фейковое содержимое) схлопнулись в один hash
      — дедупликация подтверждена и на этом слое, не только в storage.rs.
```

Новые CLI-команды (`src/bin/cli.rs`): `inspect-donor`, `build-patch`,
`apply-mix` (build-patch + сразу через `RealmBridge::apply_patch`, то есть
с проверкой процесса и бэкапом). `apply-mix` в этой сессии не гонялся живьём
(нужен собранный `realm-bridge`, а он требует .NET, которого здесь нет) —
но использует тот же проверенный `RealmBridge::apply_patch` из Этапа 2.

## Упрощение Этапа 3, которое стоит знать

`build_plan_from_donor()` в CLI сейчас заменяет **все** найденные у донора
файлы разом (нет выборочности по категориям/отдельным компонентам) — это
осознанно оставлено для Этапа 4: GUI-селектор будет строить
`Vec<ReplacementSelection>` из кликов пользователя поверх того же
`mixer::load_donor()`, `mixer.rs` уже к этому готов (`ReplacementSelection`
конструируется вызывающей стороной, а не внутри `build_patch`).

## Дальше — Этап 4

GUI на `egui`/`eframe`: боковая панель со списком скинов из `dump-skins`,
вкладки категорий на базе `grouped_by_category()`, превью-канвас (crate
`image` + `egui::Context::load_texture`), аудио-плеер (`rodio`), экран
сохранения → `apply-mix`/`RealmBridge::apply_patch` с прогресс-баром.
