import re
import subprocess

RGX = re.compile(r"(.*)-([0-9]+\.[0-9]+\.[0-9]+)")

with open("versions") as f:
    versions = f.readlines()
    versions = versions[5:]

for version in versions:
    version = version.strip()
    name, commit = version.split(" ")
    product, version = RGX.match(name).groups()

    if product in {"csharp-sdk", "ts-sdk", "pond", "rust-sdk"}:
        continue

    print(product, version, commit)

    subprocess.run(["git", "checkout", commit], check=True)
    subprocess.run(["git", "tag", f"{product}/{version}"])


subprocess.run(["git", "checkout", "master"])
