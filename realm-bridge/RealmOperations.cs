using Realms;

namespace RealmBridge;

/// <summary>
/// All Realm operations use the Dynamic API (RealmConfiguration.IsDynamic = true).
/// We do not link compiled classes from osu!lazer source code; instead we inspect
/// the schema dynamically directly from the database file.
/// </summary>
internal static class RealmOperations
{
    private const string SkinClass = "Skin";
    private const string FileClass = "File";

    public static RealmConfiguration OpenConfig(string dbPath)
    {
        if (!File.Exists(dbPath))
            throw new FileNotFoundException($"realm file not found: {dbPath}");

        return new RealmConfiguration(dbPath)
        {
            IsDynamic = true,
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
                        throw new InvalidOperationException($"unknown patch operation: '{op.Op}'");
                }
            }
        });
    }

    private static dynamic FindSkin(Realm realm, string id)
    {
        dynamic? skin = realm.DynamicApi.Find(SkinClass, Guid.Parse(id));
        if (skin is null)
            throw new InvalidOperationException($"skin with id={id} not found");
        return skin;
    }

    private static void DuplicateSkin(Realm realm, PatchOperation op)
    {
        if (op.SourceId is null || op.NewId is null)
            throw new InvalidOperationException("duplicate_skin requires source_id and new_id");

        dynamic source = FindSkin(realm, op.SourceId);
        var newId = Guid.Parse(op.NewId);

        if (realm.DynamicApi.Find(SkinClass, newId) is not null)
            throw new InvalidOperationException($"skin with id={newId} already exists");

        dynamic clone = realm.DynamicApi.CreateObject(SkinClass, newId);
        clone.Name = op.Name ?? (string)source.Name;
        clone.Creator = op.Creator ?? (string)source.Creator;
        clone.Protected = false;
        clone.InstantiationInfo = (string)source.InstantiationInfo;
        clone.Hash = string.Empty;
        clone.DeletePending = false;

        foreach (dynamic usage in source.Files)
        {
            AddFileUsage(realm, clone, (string)usage.Filename, (string)usage.File.Hash);
        }
    }

    private static void AddFile(Realm realm, PatchOperation op)
    {
        if (op.SkinId is null || op.Filename is null || op.FileHash is null)
            throw new InvalidOperationException("add_file requires skin_id, filename and file_hash");

        dynamic skin = FindSkin(realm, op.SkinId);

        RemoveFileUsageIfPresent(skin, op.Filename);
        AddFileUsage(realm, skin, op.Filename, op.FileHash);
    }

    private static void AddFileUsage(Realm realm, dynamic skin, string filename, string fileHash)
    {
        dynamic? existingFile = realm.DynamicApi.Find(FileClass, fileHash);
        dynamic file = existingFile ?? realm.DynamicApi.CreateObject(FileClass, fileHash);

        dynamic usage = realm.DynamicApi.AddEmbeddedObjectToList(skin.Files);
        usage.Filename = filename;
        usage.File = file;
    }

    private static void RemoveFile(Realm realm, PatchOperation op)
    {
        if (op.SkinId is null || op.Filename is null)
            throw new InvalidOperationException("remove_file requires skin_id and filename");

        dynamic skin = FindSkin(realm, op.SkinId);

        if (!RemoveFileUsageIfPresent(skin, op.Filename))
            throw new InvalidOperationException($"file '{op.Filename}' not found in skin {op.SkinId}");
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
            throw new InvalidOperationException("soft_delete_skin requires id");

        dynamic skin = FindSkin(realm, op.Id);

        if ((bool)skin.Protected)
            throw new InvalidOperationException($"skin {op.Id} is protected (Protected=true), deletion is forbidden");

        skin.DeletePending = true;
    }
}
