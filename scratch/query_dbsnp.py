# ./scratch/query_dbsnp.py
import urllib.request
import json
import sys

def main():
    rsid = "55886062"
    url = f"https://api.ncbi.nlm.nih.gov/variation/v0/beta/refsnp/{rsid}"
    print(f"Fetching dbSNP data from {url}...")
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        with urllib.request.urlopen(req) as response:
            data = json.loads(response.read().decode())
        
        # Save output
        out_path = "rs55886062_raw.json"
        with open(out_path, "w") as f:
            json.dump(data, f, indent=2)
        print(f"Saved raw response to {out_path}")

        # Extract basic info
        refsnp = data.get("present_obs_movements", [])
        print("\nPrimary Observations / Placements:")
        for placement in data.get("primary_snapshot_data", {}).get("placements_with_allele", []):
            seq_id = placement.get("seq_id")
            print(f"Sequence ID: {seq_id}")
            for allele in placement.get("alleles", []):
                spdi = allele.get("allele", {}).get("spdi", {})
                print(f"  SPDI: {spdi}")
                
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
