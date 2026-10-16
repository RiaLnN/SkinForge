use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use skinforge_core::{
    bridge::RealmBridge,
    categorize::ComponentCategory,
    mixer::{self, MixPlan, ReplacementSelection},
    paths, process_guard,
};

#[derive(Parser)]
#[command(
    name = "skinforge-cli",
    about = "SkinForge CLI for osu!lazer - skin mixing and Realm bridge management"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Locate osu!lazer data directory and print detected paths.
    FindLazer {
        #[arg(long)]
        lazer_path: Option<PathBuf>,
    },
    /// Check whether osu! (stable or lazer) is currently running.
    CheckRunning,
    /// Calculate SHA-256 for a file and determine its destination in files/ storage.
    HashFile {
        path: PathBuf,
        #[arg(long)]
        files_dir: PathBuf,
    },
    /// Invoke realm-bridge dump-skins to inspect installed skins.
    DumpSkins {
        #[arg(long)]
        bridge_binary: PathBuf,
        #[arg(long)]
        db: PathBuf,
    },
    /// Inspect a donor skin directory and list components by category.
    InspectDonor {
        /// Path to skin directory.
        donor: PathBuf,
        #[arg(long)]
        extract_to: Option<PathBuf>,
    },
    /// Build a Realm patch from a donor directory and export it as JSON.
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
    /// Full cycle: assemble patch from donor and apply it directly via realm-bridge.
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
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run_find_lazer(lazer_path: Option<PathBuf>) -> skinforge_core::CoreResult<()> {
    let installation = paths::resolve_lazer_root_with_override(lazer_path)?;
    println!("root:         {}", installation.root.display());
    println!("client.realm: {}", installation.client_realm.display());
    println!("files_dir:    {}", installation.files_dir.display());
    Ok(())
}

fn run_check_running() -> skinforge_core::CoreResult<()> {
    let running = process_guard::find_running_game_processes();
    if running.is_empty() {
        println!("osu! is not running — database is safe to access.");
    } else {
        for p in running {
            println!(
                "found running game process: pid={} name='{}'",
                p.pid, p.name
            );
        }
    }
    Ok(())
}

fn run_hash_file(path: &Path, files_dir: &Path) -> skinforge_core::CoreResult<()> {
    let stored = skinforge_core::storage::store_resource(files_dir, path)?;
    println!("hash:        {}", stored.hash);
    println!("destination: {}", stored.destination.display());
    println!("dedup_hit:   {}", stored.already_present);
    Ok(())
}

fn run_dump_skins(bridge_binary: &Path, db: &Path) -> skinforge_core::CoreResult<()> {
    let bridge = RealmBridge::new(bridge_binary)?;
    let skins = bridge.dump_skins(db)?;
    println!("{}", serde_json::to_string_pretty(&skins).unwrap());
    Ok(())
}

fn resolve_donor_root(
    donor: &Path,
    _extract_to: Option<PathBuf>,
) -> skinforge_core::CoreResult<PathBuf> {
    if donor.is_dir() {
        return Ok(donor.to_path_buf());
    }

    Err(skinforge_core::CoreError::BackupFailed(format!(
        "'{}' is not a directory",
        donor.display()
    )))
}

fn run_inspect_donor(donor: &Path, extract_to: Option<PathBuf>) -> skinforge_core::CoreResult<()> {
    let root = resolve_donor_root(donor, extract_to)?;
    let loaded = mixer::load_donor(&root)?;

    println!(
        "skin.ini Name:   {}",
        loaded.ini.skin_name().unwrap_or("(none)")
    );
    println!(
        "skin.ini Author: {}",
        loaded.ini.skin_author().unwrap_or("(none)")
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
    donor: &Path,
    extract_to: Option<PathBuf>,
    files_dir: &Path,
    base_skin_id: String,
    name: String,
    creator: String,
    out: &Path,
) -> skinforge_core::CoreResult<()> {
    let plan = build_plan_from_donor(donor, extract_to, files_dir, base_skin_id, name, creator)?;
    let generated = mixer::build_patch(files_dir, &plan)?;

    #[derive(serde::Serialize)]
    struct PatchFileOut<'a> {
        operations: &'a [skinforge_core::bridge::PatchOperation],
    }

    let json = serde_json::to_string_pretty(&PatchFileOut {
        operations: &generated.operations,
    })
    .unwrap();
    std::fs::write(out, json).map_err(|e| skinforge_core::CoreError::Io {
        path: out.to_path_buf(),
        source: e,
    })?;

    println!("new_skin_id: {}", generated.new_skin_id);
    println!("operations:  {}", generated.operations.len());
    println!("patch saved to {}", out.display());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_apply_mix(
    donor: &Path,
    extract_to: Option<PathBuf>,
    bridge_binary: &Path,
    db: &Path,
    files_dir: &Path,
    base_skin_id: String,
    name: String,
    creator: String,
) -> skinforge_core::CoreResult<()> {
    let plan = build_plan_from_donor(donor, extract_to, files_dir, base_skin_id, name, creator)?;
    let generated = mixer::build_patch(files_dir, &plan)?;

    let bridge = RealmBridge::new(bridge_binary)?;
    let outcome = bridge.apply_patch(db, generated.operations)?;

    println!("new skin ID:      {}", generated.new_skin_id);
    println!("database backup:  {}", outcome.backup_path.display());
    println!("result:           {}", outcome.applied_operations);
    Ok(())
}

fn build_plan_from_donor(
    donor: &Path,
    extract_to: Option<PathBuf>,
    files_dir: &Path,
    base_skin_id: String,
    name: String,
    creator: String,
) -> skinforge_core::CoreResult<MixPlan> {
    let root = resolve_donor_root(donor, extract_to)?;
    let loaded = mixer::load_donor(&root)?;

    let replacements = loaded
        .components
        .iter()
        .map(|c| ReplacementSelection {
            target_filename: c.filename.clone(),
            source_path: loaded.resource_path(&c.filename),
        })
        .collect();

    let _ = files_dir;

    Ok(MixPlan {
        base_skin_id,
        new_skin_name: name,
        new_skin_creator: creator,
        replacements,
    })
}
