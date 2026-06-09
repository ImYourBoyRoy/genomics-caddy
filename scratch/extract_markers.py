# ./scratch/extract_markers.py
import json
import glob
import os

def main():
    packs_dir = r"c:\Users\Roy\Desktop\AI\DNA_Tools\src\lib\marker-packs"
    pack_files = glob.glob(os.path.join(packs_dir, "*.json"))
    
    out = []
    for fpath in pack_files:
        if os.path.basename(fpath) == "manifest.json":
            continue
        with open(fpath, "r", encoding="utf-8") as f:
            data = json.load(f)
            for m in data.get("markers", []):
                out.append({
                    "rsid": m["rsid"],
                    "gene": m["gene"],
                    "impact": m["impact"],
                    "interpretation": m["interpretation"]
                })
                
    print(f"Total markers extracted: {len(out)}")
    with open(r"c:\Users\Roy\Desktop\AI\DNA_Tools\scratch\markers_extracted.json", "w", encoding="utf-8") as f:
        json.dump(out, f, indent=2)

if __name__ == "__main__":
    main()
