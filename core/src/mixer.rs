use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::bridge::PatchOperation;
use crate::categorize::{self, ComponentCategory, ComponentInfo};
use crate::error::{CoreError, CoreResult};
use crate::skin_ini::SkinIni;
use crate::storage;

/// Донор — распакованная папка (из `.osk` или указанная напрямую), её
/// `skin.ini` и классифицированный список всех найденных файлов ресурсов.
#[derive(Debug, Clone)]
pub struct DonorSkin {
    /// Директория, где непосредственно лежит `skin.ini` (может отличаться от
    /// переданного `root`, если архив содержал один вложенный подкаталог —
    /// частый случай для `.osk`, экспортированных вручную из ZIP).
    pub root: PathBuf,
    pub ini: SkinIni,
    pub components: Vec<ComponentInfo>,
}

impl DonorSkin {
    pub fn components_by_category(&self, category: ComponentCategory) -> Vec<&ComponentInfo> {
        self.components
            .iter()
            .filter(|c| c.category == category)
            .collect()
    }

    pub fn grouped_by_category(&self) -> HashMap<ComponentCategory, Vec<&ComponentInfo>> {
        let mut map: HashMap<ComponentCategory, Vec<&ComponentInfo>> = HashMap::new();
        for component in &self.components {
            map.entry(component.category).or_default().push(component);
        }
        map
    }

    /// Абсолютный путь на диске к файлу ресурса донора по его имени (как оно
    /// хранится в `ComponentInfo::filename` — относительный путь от `root`,
    /// включая вложенные поддиректории вроде `sounds/hitnormal.wav`).
    pub fn resource_path(&self, relative_filename: &str) -> PathBuf {
        self.root.join(relative_filename)
    }
}

/// Загружает донора из уже распакованной (или изначально папочной) директории:
/// ищет `skin.ini` (нечувствительно к регистру, на случай архивов с одним
/// вложенным подкаталогом верхнего уровня — тогда эффективным `root` становится
/// именно та вложенная папка), парсит его и классифицирует остальные файлы.
pub fn load_donor(extracted_root: &Path) -> CoreResult<DonorSkin> {
    let ini_path = find_skin_ini(extracted_root).ok_or_else(|| {
        CoreError::BackupFailed(format!(
            "skin.ini не найден в '{}'",
            extracted_root.display()
        ))
    })?;

    let effective_root = ini_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| extracted_root.to_path_buf());

    let ini = SkinIni::read_from_file(&ini_path)?;
    let components = scan_components(&effective_root, &ini_path)?;

    Ok(DonorSkin {
        root: effective_root,
        ini,
        components,
    })
}

fn find_skin_ini(root: &Path) -> Option<PathBuf> {
    // Сначала — прямое попадание в корне (самый частый случай).
    let direct = root.join("skin.ini");
    if direct.is_file() {
        return Some(direct);
    }

    // Иначе ищем на пару уровней вглубь (архив с одним вложенным каталогом,
    // изредка — два, если кто-то заархивировал родительскую папку целиком).
    for entry in walkdir::WalkDir::new(root).min_depth(1).max_depth(3) {
        let Ok(entry) = entry else { continue };
        if entry.file_type().is_file() && entry.file_name().eq_ignore_ascii_case("skin.ini") {
            return Some(entry.path().to_path_buf());
        }
    }

    None
}

fn scan_components(effective_root: &Path, ini_path: &Path) -> CoreResult<Vec<ComponentInfo>> {
    let mut components = Vec::new();

    for entry in walkdir::WalkDir::new(effective_root).min_depth(1) {
        let entry = entry.map_err(|e| {
            CoreError::BackupFailed(format!("ошибка обхода директории донора: {e}"))
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path() == ini_path {
            continue; // сам skin.ini — не ресурс для категоризации
        }

        let relative = entry
            .path()
            .strip_prefix(effective_root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");

        // Категоризатор смотрит на имя файла (последний компонент пути) —
        // структура вложенных папок вроде `sounds/` на классификацию не влияет,
        // но полный относительный путь сохраняется как filename для патча.
        let basename = entry.file_name().to_string_lossy().to_string();
        let mut info = categorize::classify(&basename);
        info.filename = relative;
        components.push(info);
    }

    Ok(components)
}

/// Одна выбранная пользователем замена: какой файл донора (`source_path` —
/// абсолютный путь на диске) должен лечь в целевой скин под именем `target_filename`
/// (обычно совпадает с исходным относительным именем ресурса).
#[derive(Debug, Clone)]
pub struct ReplacementSelection {
    pub target_filename: String,
    pub source_path: PathBuf,
}

/// План сборки: какой существующий скин клонировать как базу, метаданные
/// нового скина и список замен поверх скопированных файлов базы.
#[derive(Debug, Clone)]
pub struct MixPlan {
    pub base_skin_id: String,
    pub new_skin_name: String,
    pub new_skin_creator: String,
    pub replacements: Vec<ReplacementSelection>,
}

/// Результат генерации патча: id нового скина (уже сгенерированный, чтобы
/// вызывающая сторона знала его заранее — например, для превью в GUI до
/// фактического применения) и сами операции для `RealmBridge::apply_patch`.
pub struct GeneratedPatch {
    pub new_skin_id: String,
    pub operations: Vec<PatchOperation>,
}

/// Хэширует и копирует в `files_dir` каждый файл замены из плана, затем
/// собирает список операций патча: `duplicate_skin` (клонирование базы под
/// новым Guid) + по одному `add_file` на каждую замену. Само применение к базе
/// (запуск `realm-bridge`) — дело `RealmBridge::apply_patch`, эта функция
/// только готовит операции и физически размещает файлы в хранилище.
pub fn build_patch(files_dir: &Path, plan: &MixPlan) -> CoreResult<GeneratedPatch> {
    let new_skin_id = uuid::Uuid::new_v4().to_string();

    let mut operations = vec![PatchOperation::duplicate_skin(
        plan.base_skin_id.clone(),
        new_skin_id.clone(),
        plan.new_skin_name.clone(),
        plan.new_skin_creator.clone(),
    )];

    for replacement in &plan.replacements {
        let stored = storage::store_resource(files_dir, &replacement.source_path)?;
        operations.push(PatchOperation::add_file(
            new_skin_id.clone(),
            replacement.target_filename.clone(),
            stored.hash,
        ));
    }

    Ok(GeneratedPatch {
        new_skin_id,
        operations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "osu-mixer-mixer-test-{name}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn loads_donor_from_flat_directory() {
        let root = temp_dir("flat");
        fs::write(
            root.join("skin.ini"),
            "[General]\nName: Flat Donor\nAuthor: me\n",
        )
        .unwrap();
        fs::write(root.join("cursor.png"), b"fake").unwrap();
        fs::create_dir_all(root.join("sounds")).unwrap();
        fs::write(root.join("sounds/hitnormal.wav"), b"fake").unwrap();

        let donor = load_donor(&root).unwrap();
        assert_eq!(donor.ini.skin_name(), Some("Flat Donor"));
        assert_eq!(donor.components.len(), 2);

        let cursor = donor
            .components
            .iter()
            .find(|c| c.filename == "cursor.png")
            .unwrap();
        assert_eq!(cursor.category, ComponentCategory::Cursor);

        let hitnormal = donor
            .components
            .iter()
            .find(|c| c.filename.ends_with("hitnormal.wav"))
            .unwrap();
        assert_eq!(hitnormal.category, ComponentCategory::Audio);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn loads_donor_from_nested_single_subfolder() {
        // Частый случай: архив запакован так, что внутри один каталог верхнего
        // уровня с именем скина, а skin.ini лежит уже в нём.
        let root = temp_dir("nested");
        let nested = root.join("My Donor Skin");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("skin.ini"), "[General]\nName: Nested Donor\n").unwrap();
        fs::write(nested.join("hitcircle.png"), b"fake").unwrap();

        let donor = load_donor(&root).unwrap();
        assert_eq!(donor.root, nested);
        assert_eq!(donor.ini.skin_name(), Some("Nested Donor"));
        assert_eq!(donor.components.len(), 1);

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn missing_skin_ini_is_an_error() {
        let root = temp_dir("missing-ini");
        fs::write(root.join("cursor.png"), b"fake").unwrap();

        let result = load_donor(&root);
        assert!(result.is_err());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn build_patch_generates_duplicate_plus_add_file_ops() {
        let work_dir = temp_dir("patch");
        let files_dir = work_dir.join("files");
        let source_file = work_dir.join("cursor.png");
        fs::write(&source_file, b"fake cursor bytes").unwrap();

        let plan = MixPlan {
            base_skin_id: "11111111-1111-1111-1111-111111111111".to_string(),
            new_skin_name: "My Mix".to_string(),
            new_skin_creator: "me".to_string(),
            replacements: vec![ReplacementSelection {
                target_filename: "cursor.png".to_string(),
                source_path: source_file,
            }],
        };

        let generated = build_patch(&files_dir, &plan).unwrap();
        assert_eq!(generated.operations.len(), 2); // duplicate_skin + 1 add_file
        assert!(!generated.new_skin_id.is_empty());

        // Файл действительно лёг в content-addressed хранилище.
        let stored_files: Vec<_> = walkdir::WalkDir::new(&files_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();
        assert_eq!(stored_files.len(), 1);

        fs::remove_dir_all(&work_dir).ok();
    }
}
