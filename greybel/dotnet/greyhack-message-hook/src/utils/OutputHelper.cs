using System.Collections.Generic;
using Miniscript;
using Newtonsoft.Json;

namespace GreyHackMessageHook
{
    public static class OutputHelper
    {
        public static string ToValMapsJson(List<ValMap> valMaps)
        {
            Dictionary<string, string> output = new Dictionary<string, string>();

            foreach (var valMap in valMaps)
            {
                if (valMap == null)
                    continue;
                foreach (Value key in valMap.Keys)
                {
                    if (key == null)
                        continue;
                    string keyStr = key.ToString(null);
                    if (!valMap.map.ContainsKey(key))
                        continue;
                    Value val = valMap.map[key];
                    if (val == null)
                    {
                        output[keyStr] = "null";
                    }
                    else
                    {
                        output[keyStr] = val.CodeForm(null, 1);
                    }
                }
            }

            return JsonConvert.SerializeObject(output);
        }
    }
}
