using System.Text.Json;
using RealmBridge;

// Контракт с вызывающей стороной (Rust):
//   - Всегда ровно ОДИН JSON-объект в stdout, независимо от исхода.
//   - Код возврата процесса: 0 = success:true, 1 = success:false (см. поле "error").
//   - Любой не-JSON вывод (стектрейсы и т.п.) НИКОГДА не должен попасть в stdout,
//     только в stderr — для отладки человеком, не для парсинга.
//
// Команды:
//   realm-bridge dump-skins --db <path/to/client.realm>
//   realm-bridge apply-patch --db <path/to/client.realm> --patch <path/to/patch.json>

var jsonOptions = new JsonSerializerOptions
{
    WriteIndented = false,
    PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
};

try
{
    if (args.Length == 0)
        return Fail("не указана команда. Ожидается 'dump-skins' или 'apply-patch'.");

    string command = args[0];
    var options = ParseOptions(args.AsSpan(1).ToArray());

    switch (command)
    {
        case "dump-skins":
        {
            string db = RequireOption(options, "db");
            var skins = RealmOperations.DumpSkins(db);
            return Succeed(skins);
        }

        case "apply-patch":
        {
            string db = RequireOption(options, "db");
            string patchPath = RequireOption(options, "patch");

            if (!File.Exists(patchPath))
                return Fail($"файл патча не найден: {patchPath}");

            var patchJson = File.ReadAllText(patchPath);
            var patch = JsonSerializer.Deserialize<PatchFile>(patchJson, jsonOptions)
                        ?? throw new InvalidOperationException("не удалось разобрать patch.json");

            if (patch.Operations.Count == 0)
                return Fail("патч не содержит ни одной операции");

            RealmOperations.ApplyPatch(db, patch);
            return Succeed(new { applied_operations = patch.Operations.Count });
        }

        default:
            return Fail($"неизвестная команда '{command}'");
    }
}
catch (Exception ex)
{
    // Полный стектрейс — только в stderr, чтобы не засорять stdout, который парсит Rust.
    Console.Error.WriteLine(ex);
    return Fail(ex.Message);
}

int Succeed(object data)
{
    var result = new BridgeResult { Success = true, Data = data };
    Console.WriteLine(JsonSerializer.Serialize(result, jsonOptions));
    return 0;
}

int Fail(string message)
{
    var result = new BridgeResult { Success = false, Error = message };
    Console.WriteLine(JsonSerializer.Serialize(result, jsonOptions));
    return 1;
}

Dictionary<string, string> ParseOptions(string[] rest)
{
    var map = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    for (int i = 0; i < rest.Length - 1; i++)
    {
        if (rest[i].StartsWith("--", StringComparison.Ordinal))
        {
            string key = rest[i][2..];
            map[key] = rest[i + 1];
            i++;
        }
    }
    return map;
}

string RequireOption(Dictionary<string, string> options, string key)
{
    if (!options.TryGetValue(key, out var value) || string.IsNullOrWhiteSpace(value))
        throw new InvalidOperationException($"отсутствует обязательный параметр --{key}");
    return value;
}
