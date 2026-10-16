use std::path::PathBuf;

use clap::{Parser, Subcommand};
use osu_mixer_core::{
    bridge::RealmBridge,
    categorize::ComponentCategory,
    mixer::{self, MixPlan, ReplacementSelection},
    paths, process_guard,
};

#[derive(Parser)]
#[command(
    name = "osu-mixer",
    about = "Ядро osu-mixer для lazer — Этапы 2-3 (файловый менеджер, обёртка моста, микширование)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Найти директорию данных lazer и вывести пути.
    FindLazer {
        #[arg(long)]
        lazer_path: Option<PathBuf>,
    },
    /// Проверить, не запущен ли osu! (stable или lazer).
    CheckRunning,
    /// Посчитать SHA-256 файла и путь, по которому он лёг бы в files/.
    HashFile {
        path: PathBuf,
        #[arg(long)]
        files_dir: PathBuf,
    },
    /// Вызвать realm-bridge dump-skins через обёртку.
    DumpSkins {
        #[arg(long)]
        bridge_binary: PathBuf,
        #[arg(long)]
        db: PathBuf,
    },
    /// Загрузить донора (папка или .osk) и вывести найденные компоненты по категориям.
    InspectDonor {
        /// Путь к .osk-архиву или к уже распакованной папке скина.
        donor: PathBuf,
        /// Куда распаковывать .osk, если donor — архив (по умолчанию — временная директория).
        #[arg(long)]
        extract_to: Option<PathBuf>,
    },
    /// Собрать патч из донора (клонировать base-skin + заменить все найденные
    /// файлы донора) и сохранить как JSON, готовый для `realm-bridge apply-patch`.
    BuildPatch {
        donor: PathBuf,
        #[arg(long)]
        extract_to: Option<PathBuf>,
        #[arg(long)]
        files_dir: PathBuf,
        #[arg(long)]
        base_skin_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        creator: String,
        #[arg(long)]
        out: PathBuf,
    },
    /// Полный цикл: собрать патч из донора и сразу применить его через realm-bridge
    /// (с проверкой запущенного процесса и бэкапом — см. RealmBridge::apply_patch).
    ApplyMix {
        donor: PathBuf,
        #[arg(long)]
        extract_to: Option<PathBuf>,
        #[arg(long)]
        bridge_binary: PathBuf,
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        files_dir: PathBuf,
        #[arg(long)]
        base_skin_id: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        creator: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::FindLazer { lazer_path } => run_find_lazer(lazer_path),
        Commands::CheckRunning => run_check_running(),
        Commands::HashFile { path, files_dir } => run_hash_file(&path, &files_dir),
        Commands::DumpSkins { bridge_binary, db } => run_dump_skins(&bridge_binary, &db),
        Commands::InspectDonor { donor, extract_to } => run_inspect_donor(&donor, extract_to),
        Commands::BuildPatch {
            donor,
            extract_to,
            files_dir,
            base_skin_id,
            name,
            creator,
            out,
        } => run_build_patch(
            &donor,
            extract_to,
            &files_dir,
            base_skin_id,
            name,
            creator,
            &out,
        ),
        Commands::ApplyMix {
            donor,
            extract_to,
            bridge_binary,
            db,
            files_dir,
            base_skin_id,
            name,
            creator,
        } => run_apply_mix(
            &donor,
            extract_to,
            &bridge_binary,
            &db,
            &files_dir,
            base_skin_id,
            name,
            creator,
        ),
    };

    if let Err(err) = result {
        eprintln!("ошибка: {err}");
        std::process::exit(1);
    }
}

fn run_find_lazer(lazer_path: Option<PathBuf>) -> osu_mixer_core::CoreResult<()> {
    let installation = paths::resolve_lazer_root_with_override(lazer_path)?;
    println!("root:         {}", installation.root.display());
    println!("client.realm: {}", installation.client_realm.display());
    println!("files_dir:    {}", installation.files_dir.display());
    Ok(())
}

fn run_check_running() -> osu_mixer_core::CoreResult<()> {
    let running = process_guard::find_running_game_processes();
    if running.is_empty() {
        println!("osu! не обнаружен — можно работать с базой.");
    } else {
        for p in running {
            println!("найден процесс: pid={} name='{}'", p.pid, p.name);
        }
    }
    Ok(())
}

fn run_hash_file(path: &PathBuf, files_dir: &PathBuf) -> osu_mixer_core::CoreResult<()> {
    let stored = osu_mixer_core::storage::store_resource(files_dir, path)?;
    println!("hash:        {}", stored.hash);
    println!("destination: {}", stored.destination.display());
    println!("dedup_hit:   {}", stored.already_present);
    Ok(())
}

fn run_dump_skins(bridge_binary: &PathBuf, db: &PathBuf) -> osu_mixer_core::CoreResult<()> {
    let bridge = RealmBridge::new(bridge_binary)?;
    let skins = bridge.dump_skins(db)?;
    println!("{}", serde_json::to_string_pretty(&skins).unwrap());
    Ok(())
}

/// Если donor — файл с расширением .osk, распаковывает его (в extract_to или
/// во временную директорию) и возвращает путь к распакованной папке. Если это
/// уже директория — возвращает как есть.
fn resolve_donor_root(
    donor: &PathBuf,
    _extract_to: Option<PathBuf>,
) -> osu_mixer_core::CoreResult<PathBuf> {
    if donor.is_dir() {
        return Ok(donor.clone());
    }

    Err(osu_mixer_core::CoreError::BackupFailed(format!(
        "'{}' — не директория",
        donor.display()
    )))
}

fn run_inspect_donor(
    donor: &PathBuf,
    extract_to: Option<PathBuf>,
) -> osu_mixer_core::CoreResult<()> {
    let root = resolve_donor_root(donor, extract_to)?;
    let loaded = mixer::load_donor(&root)?;

    println!(
        "skin.ini Name:   {}",
        loaded.ini.skin_name().unwrap_or("(нет)")
    );
    println!(
        "skin.ini Author: {}",
        loaded.ini.skin_author().unwrap_or("(нет)")
    );
    println!("combo colours:   {:?}", loaded.ini.combo_colours());
    println!();

    for category in [
        ComponentCategory::Gameplay,
        ComponentCategory::Cursor,
        ComponentCategory::Audio,
        ComponentCategory::Interface,
        ComponentCategory::Other,
    ] {
        let items = loaded.components_by_category(category);
        println!("{category:?} ({}):", items.len());
        for item in items {
            println!("  {} -> {}", item.filename, item.component_key);
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_build_patch(
    donor: &PathBuf,
    extract_to: Option<PathBuf>,
    files_dir: &PathBuf,
    base_skin_id: String,
    name: String,
    creator: String,
    out: &PathBuf,
) -> osu_mixer_core::CoreResult<()> {
    let plan = build_plan_from_donor(donor, extract_to, files_dir, base_skin_id, name, creator)?;
    let generated = mixer::build_patch(files_dir, &plan)?;

    #[derive(serde::Serialize)]
    struct PatchFileOut<'a> {
        operations: &'a [osu_mixer_core::bridge::PatchOperation],
    }

    let json = serde_json::to_string_pretty(&PatchFileOut {
        operations: &generated.operations,
    })
    .unwrap();
    std::fs::write(out, json).map_err(|e| osu_mixer_core::CoreError::Io {
        path: out.clone(),
        source: e,
    })?;

    println!("new_skin_id: {}", generated.new_skin_id);
    println!("операций:    {}", generated.operations.len());
    println!("патч сохранён в {}", out.display());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_apply_mix(
    donor: &PathBuf,
    extract_to: Option<PathBuf>,
    bridge_binary: &PathBuf,
    db: &PathBuf,
    files_dir: &PathBuf,
    base_skin_id: String,
    name: String,
    creator: String,
) -> osu_mixer_core::CoreResult<()> {
    let plan = build_plan_from_donor(donor, extract_to, files_dir, base_skin_id, name, creator)?;
    let generated = mixer::build_patch(files_dir, &plan)?;

    let bridge = RealmBridge::new(bridge_binary)?;
    let outcome = bridge.apply_patch(db, generated.operations)?;

    println!("новый скин:  {}", generated.new_skin_id);
    println!("бэкап базы:  {}", outcome.backup_path.display());
    println!("результат:   {}", outcome.applied_operations);
    Ok(())
}

fn build_plan_from_donor(
    donor: &PathBuf,
    extract_to: Option<PathBuf>,
    files_dir: &PathBuf,
    base_skin_id: String,
    name: String,
    creator: String,
) -> osu_mixer_core::CoreResult<MixPlan> {
    let root = resolve_donor_root(donor, extract_to)?;
    let loaded = mixer::load_donor(&root)?;

    // Stage 3: заменяем ВСЕ найденные у донора файлы. Выборочная замена по
    // категориям/отдельным компонентам — задача GUI-селектора на Этапе 4,
    // который будет строить List<ReplacementSelection> из кликов пользователя
    // поверх того же mixer::load_donor().
    let replacements = loaded
        .components
        .iter()
        .map(|c| ReplacementSelection {
            target_filename: c.filename.clone(),
            source_path: loaded.resource_path(&c.filename),
        })
        .collect();

    let _ = files_dir; // files_dir используется только в build_patch/apply_patch, не здесь

    Ok(MixPlan {
        base_skin_id,
        new_skin_name: name,
        new_skin_creator: creator,
        replacements,
    })
}
