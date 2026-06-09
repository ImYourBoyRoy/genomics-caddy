# ./scratch/test_show_models.py
import urllib.request
import json

url = "http://192.168.1.21:11434"

def make_request(path, data=None):
    req_url = f"{url}{path}"
    headers = {"Content-Type": "application/json"}
    req_data = json.dumps(data).encode('utf-8') if data else None
    req = urllib.request.Request(req_url, data=req_data, headers=headers, method="POST" if data else "GET")
    try:
        with urllib.request.urlopen(req) as res:
            return json.loads(res.read().decode('utf-8'))
    except Exception as e:
        print(f"Error requesting {path}: {e}")
        return None

def main():
    print("Fetching models from /api/tags...")
    tags_res = make_request("/api/tags")
    if not tags_res or "models" not in tags_res:
        print("Failed to fetch models from /api/tags")
        return
        
    models = tags_res["models"]
    print(f"Found {len(models)} models:")
    for m in models:
        name = m["name"]
        print(f"\n--- Model: {name} ---")
        print("Details from /api/tags details:")
        print(json.dumps(m.get("details", {}), indent=2))
        
        print("Fetching /api/show...")
        show_res = make_request("/api/show", {"name": name})
        if show_res:
            # Print important keys
            details = show_res.get("details", {})
            model_info = show_res.get("model_info", {})
            print("Details from /api/show details:")
            print(json.dumps(details, indent=2))
            
            # Print model_info keys and some values
            print("Model Info keys from /api/show:")
            info_summary = {}
            for k, v in model_info.items():
                if any(x in k for x in ["type", "task", "architecture", "input", "output", "model", "capabilities"]):
                    info_summary[k] = v
                elif "context_length" in k:
                    info_summary[k] = v
            print(json.dumps(info_summary, indent=2))
            
            # Let's print the entire model_info if it's small, or keys
            print("Keys list in model_info:")
            print(list(model_info.keys()))
            
            # Check for project parameters or template
            parameters = show_res.get("parameters", "")
            print("Parameters snippet (first 100 chars):")
            print(repr(parameters[:100]))

if __name__ == "__main__":
    main()
