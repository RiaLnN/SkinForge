using Realms;

namespace RealmBridge;

/// <summary>
/// Вся работа с Realm идёт через Dynamic API (RealmConfiguration.IsDynamic = true).
/// Мы НЕ линкуем скомпилированные классы SkinInfo/RealmNamedFileUsage/RealmFile из
/// исходников osu!lazer — вместо этого читаем схему прямо из файла базы и обращаемся
/// к полям по именам через `dynamic`. Так мост не ломается при обновлении схемы игры
/// (пока сами имена классов/полей "Skin"/"Name"/"Files"/"File"/"Hash" не переименуют).
///
/// Схема на момент написания (ppy/osu, osu.Game.Skinning.SkinInfo и соседние файлы):
///   class "Skin" (MapTo из SkinInfo): ID:Guid(pk), Name:string, Creator:string,
///       Protected:bool, InstantiationInfo:string, Hash:string, DeletePending:bool,
///       Files:IList&lt;RealmNamedFileUsage&gt; (embedded)
///   RealmNamedFileUsage (embedded): Filename:string, File:RealmFile
///   class "File" (MapTo из RealmFile): Hash:string(pk)
/// </summary>
internal static class RealmOperations
{
    private const string SkinClass = "Skin";
    private const string FileClass = "File";

    public static RealmConfiguration OpenConfig(string dbPath)
    {
        if (!File.Exists(dbPath))
            throw new FileNotFoundException($"realm-файл не найден: {dbPath}");

        return new RealmConfiguration(dbPath)
        {
            IsDynamic = true,
            // Схему не задаём намеренно — Dynamic API вычитывает её из файла.
        };
    }

    public static List<SkinDto> DumpSkins(string dbPath)
    {
        using var realm = Realm.GetInstance(OpenConfig(dbPath));

        var result = new List<SkinDto>();

        foreach (dynamic skin in realm.DynamicApi.All(SkinClass))
        {
            var dto = new SkinDto
            {
                Id = skin.ID.ToString(),
                Name = (string)skin.Name,
                Creator = (string)skin.Creator,
                Protected = (bool)skin.Protected,
                InstantiationInfo = (string)skin.InstantiationInfo,
                Hash = skin.Hash is null ? string.Empty : (string)skin.Hash,
                DeletePending = (bool)skin.DeletePending,
            };

            foreach (dynamic usage in skin.Files)
            {
                dto.Files.Add(new SkinFileDto
                {
                    Filename = (string)usage.Filename,
                    FileHash = (string)usage.File.Hash,
                });
            }

            result.Add(dto);
        }

        return result;
    }

    public static void ApplyPatch(string dbPath, PatchFile patch)
    {
        using var realm = Realm.GetInstance(OpenConfig(dbPath));

        realm.Write(() =>
        {
            foreach (var op in patch.Operations)
            {
                switch (op.Op)
                {
                    case "duplicate_skin":
                        DuplicateSkin(realm, op);
                        break;

                    case "add_file":
                        AddFile(realm, op);
                        break;

                    case "remove_file":
                        RemoveFile(realm, op);
                        break;

                    case "soft_delete_skin":
                        SoftDeleteSkin(realm, op);
                        break;

                    default:
                        throw new InvalidOperationException($"неизвестная операция патча: '{op.Op}'");
                }
            }
        });
        // realm.Write оборачивает делегат в транзакцию: при любом брошенном исключении
        // транзакция откатывается целиком и на диск ничего не попадает.
    }

    private static dynamic FindSkin(Realm realm, string id)
    {
        dynamic? skin = realm.DynamicApi.Find(SkinClass, Guid.Parse(id));
        if (skin is null)
            throw new InvalidOperationException($"скин с id={id} не найден");
        return skin;
    }

    private static void DuplicateSkin(Realm realm, PatchOperation op)
    {
        if (op.SourceId is null || op.NewId is null)
            throw new InvalidOperationException("duplicate_skin требует source_id и new_id");

        dynamic source = FindSkin(realm, op.SourceId);
        var newId = Guid.Parse(op.NewId);

        if (realm.DynamicApi.Find(SkinClass, newId) is not null)
            throw new InvalidOperationException($"скин с id={newId} уже существует");

        dynamic clone = realm.DynamicApi.CreateObject(SkinClass, newId);
        clone.Name = op.Name ?? (string)source.Name;
        clone.Creator = op.Creator ?? (string)source.Creator;
        clone.Protected = false; // копия никогда не защищена от удаления/правок
        clone.InstantiationInfo = (string)source.InstantiationInfo;
        clone.Hash = string.Empty; // пересчитывается движком игры при следующем запуске
        clone.DeletePending = false;

        // Клонируем список файлов: embedded-объекты нельзя переиспользовать между
        // родителями, поэтому для каждой записи создаём новый RealmNamedFileUsage,
        // указывающий на тот же (уже существующий, дедуплицированный по хэшу) File.
        foreach (dynamic usage in source.Files)
        {
            AddFileUsage(realm, clone, (string)usage.Filename, (string)usage.File.Hash);
        }
    }

    private static void AddFile(Realm realm, PatchOperation op)
    {
        if (op.SkinId is null || op.Filename is null || op.FileHash is null)
            throw new InvalidOperationException("add_file требует skin_id, filename и file_hash");

        dynamic skin = FindSkin(realm, op.SkinId);

        // Если файл с таким именем уже привязан к скину — сначала убираем старую
        // запись (замена), чтобы не плодить дубликаты Filename в списке.
        RemoveFileUsageIfPresent(skin, op.Filename);

        AddFileUsage(realm, skin, op.Filename, op.FileHash);
    }

    private static void AddFileUsage(Realm realm, dynamic skin, string filename, string fileHash)
    {
        // Content-addressed storage: если File с таким хэшем уже есть в базе — переиспользуем,
        // иначе создаём новую запись. Физическое копирование байтов в files/x/xy/ — забота
        // вызывающей стороны (Rust), этот метод только регистрирует ссылку в базе.
        dynamic? existingFile = realm.DynamicApi.Find(FileClass, fileHash);
        dynamic file = existingFile ?? realm.DynamicApi.CreateObject(FileClass, fileHash);

        dynamic usage = realm.DynamicApi.AddEmbeddedObjectToList(skin.Files);
        usage.Filename = filename;
        usage.File = file;
    }

    private static void RemoveFile(Realm realm, PatchOperation op)
    {
        if (op.SkinId is null || op.Filename is null)
            throw new InvalidOperationException("remove_file требует skin_id и filename");

        dynamic skin = FindSkin(realm, op.SkinId);

        if (!RemoveFileUsageIfPresent(skin, op.Filename))
            throw new InvalidOperationException($"файл '{op.Filename}' не найден у скина {op.SkinId}");
    }

    private static bool RemoveFileUsageIfPresent(dynamic skin, string filename)
    {
        bool removed = false;
        for (int i = skin.Files.Count - 1; i >= 0; i--)
        {
            dynamic usage = skin.Files[i];
            if (string.Equals((string)usage.Filename, filename, StringComparison.OrdinalIgnoreCase))
            {
                skin.Files.RemoveAt(i);
                removed = true;
            }
        }
        return removed;
    }

    private static void SoftDeleteSkin(Realm realm, PatchOperation op)
    {
        if (op.Id is null)
            throw new InvalidOperationException("soft_delete_skin требует id");

        dynamic skin = FindSkin(realm, op.Id);

        if ((bool)skin.Protected)
            throw new InvalidOperationException($"скин {op.Id} защищён (Protected=true), удаление запрещено");

        // Мягкое удаление, как это делает сама игра (ISoftDelete) — не трогаем файлы
        // физически и не рвём ссылки, просто помечаем на исключение из выборок.
        skin.DeletePending = true;
    }
}
