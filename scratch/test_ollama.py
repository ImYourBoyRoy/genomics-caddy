# ./scratch/test_ollama.py
import urllib.request
import urllib.error
import json

payload = {
    "model": "llama3.1:latest",
    "messages": [{"role": "user", "content": "hello"}],
    "stream": False
}

req = urllib.request.Request(
    "http://192.168.1.21:11434/api/chat",
    data=json.dumps(payload).encode(),
    headers={"Content-Type": "application/json"}
)

try:
    print("Sending request to remote Ollama...")
    with urllib.request.urlopen(req) as response:
        print("Success! Response:")
        print(response.read().decode())
except urllib.error.HTTPError as e:
    print(f"HTTP Error {e.code}:")
    print(e.read().decode())
except Exception as e:
    print(f"Connection Exception: {e}")
