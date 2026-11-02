"""Reject stale private preview builds; Master Assistant publication authorized 2 November 2026."""
from pathlib import Path
import io, zipfile
MARKER = b"LEMON_PRIVATE_MASTER_ASSISTANT_BUILD"
def require_public_binary(path):
    if private_binary(Path(path).read_bytes()):
        raise SystemExit("Private Master Assistant build: publication is blocked until Michael explicitly approves it.")
def private_source(name):
    name=name.replace("\\","/")
    return name == "AGENTS.md" or name.endswith("/WORKFLOW.md") or name.endswith("/MAC_RELEASE.md")  # Local operator notes are not corresponding application source.

def private_binary(data):
    return MARKER in data or b"LEMON_UNBUNDLED_MASTER_ASSISTANT_BUILD" in data

def require_public_artifact(path):
    path=Path(path)
    def inspect(name,data,depth=0):
        normalized=name.replace("\\","/")
        relative=normalized.removeprefix("Source/")
        if private_source(relative):
            raise SystemExit("Private Master Assistant files found in publication package.")
        if normalized.endswith('.exe') or normalized.rsplit('/',1)[-1]=='lemonstudio':
            if private_binary(data): raise SystemExit("Private Master Assistant binary cannot be published.")
        if normalized.lower().endswith('.zip'):
            if depth>4: raise SystemExit("Publication archive nesting is too deep to verify.")
            with zipfile.ZipFile(io.BytesIO(data)) as z:
                for item in z.infolist():
                    if not item.is_dir(): inspect(item.filename,z.read(item),depth+1)
    inspect(path.name,path.read_bytes())
