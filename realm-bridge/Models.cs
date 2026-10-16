using System.Text.Json.Serialization;

namespace RealmBridge;

// ---------- Вывод команды dump-skins ----------

internal sealed class SkinDto
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("creator")]
    public string Creator { get; set; } = string.Empty;

    [JsonPropertyName("protected")]
    public bool Protected { get; set; }

    [JsonPropertyName("instantiation_info")]
    public string InstantiationInfo { get; set; } = string.Empty;

    [JsonPropertyName("hash")]
    public string Hash { get; set; } = string.Empty;

    [JsonPropertyName("delete_pending")]
    public bool DeletePending { get; set; }

    [JsonPropertyName("files")]
    public List<SkinFileDto> Files { get; set; } = new();
}

internal sealed class SkinFileDto
{
    [JsonPropertyName("filename")]
    public string Filename { get; set; } = string.Empty;

    [JsonPropertyName("file_hash")]
    public string FileHash { get; set; } = string.Empty;
}

// ---------- Общий конверт результата ----------

internal sealed class BridgeResult
{
    [JsonPropertyName("success")]
    public bool Success { get; set; }

    [JsonPropertyName("error")]
    public string? Error { get; set; }

    [JsonPropertyName("data")]
    public object? Data { get; set; }
}

// ---------- Входной формат патча (apply-patch) ----------
//
// Пример файла патча:
// {
//   "operations": [
//     { "op": "duplicate_skin", "source_id": "GUID", "new_id": "GUID",
//       "name": "My mixed skin", "creator": "me" },
//     { "op": "add_file", "skin_id": "GUID", "filename": "cursor.png",
//       "file_hash": "abcd...sha256hex" },
//     { "op": "remove_file", "skin_id": "GUID", "filename": "cursor-trail.png" },
//     { "op": "soft_delete_skin", "id": "GUID" }
//   ]
// }
//
// Все операции одного файла патча применяются в ОДНОЙ write-транзакции:
// либо все успешно, либо ни одна (realm.Write откатывается целиком при исключении).

internal sealed class PatchFile
{
    [JsonPropertyName("operations")]
    public List<PatchOperation> Operations { get; set; } = new();
}

internal sealed class PatchOperation
{
    [JsonPropertyName("op")]
    public string Op { get; set; } = string.Empty;

    // duplicate_skin
    [JsonPropertyName("source_id")]
    public string? SourceId { get; set; }

    [JsonPropertyName("new_id")]
    public string? NewId { get; set; }

    [JsonPropertyName("name")]
    public string? Name { get; set; }

    [JsonPropertyName("creator")]
    public string? Creator { get; set; }

    // add_file / remove_file / soft_delete_skin
    [JsonPropertyName("skin_id")]
    public string? SkinId { get; set; }

    [JsonPropertyName("id")]
    public string? Id { get; set; }

    [JsonPropertyName("filename")]
    public string? Filename { get; set; }

    [JsonPropertyName("file_hash")]
    public string? FileHash { get; set; }
}
