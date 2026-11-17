#!/usr/bin/env python3
"""Extract names/indices only from pinned game definitions; never export game scripts.

Gene ordering and template extraction adapted from ck3-save-updater's dna.py,
copyright (c) 2026 Dustin, GPL-3.0-or-later. See THIRD_PARTY_NOTICES.md.
Run: python scripts/build-compatibility-data.py --definitions-repo ../ck3-mod-base
"""
import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

SNAPSHOTS = {
    "1.16.1": "58aec95f6", "1.16.2.3": "bc258c75c",
    "1.17.0.1": "d878f86d1", "1.17.1": "872884ce5",
    "1.18.0.1": "4dde28f60", "1.18.0.2": "be794ac59",
    "1.18.1": "c6ec9f95d", "1.18.1.1": "a7fc96def",
    "1.18.2": "27abc8e83", "1.18.3.1": "e20e42afc", "1.18.4": "a7d90e760",
    "1.19.0.6": "3b2b243d9", "1.20.0.3": "0ec13350c",
}
TOKEN = re.compile(r'#[^\n]*|"(?:[^"\\]|\\.)*"|[{}=]|[^\s{}="#]+')


def parse(text):
    toks = [m.group() for m in TOKEN.finditer(text) if not m.group().startswith("#")]
    def body(i, nested=False):
        out = []
        while i < len(toks):
            key = toks[i]
            if key == "}":
                if not nested:
                    raise ValueError("unexpected closing brace")
                return out, i + 1
            if i + 2 < len(toks) and toks[i + 1] == "=":
                value = toks[i + 2]
                if value == "{":
                    value, i = body(i + 3, True)
                else:
                    i += 3
                out.append((key, value))
            elif key == "{":
                value, i = body(i + 1, True)
                out.append((None, value))
            else:
                out.append((None, key))
                i += 1
        if nested:
            raise ValueError("unclosed brace")
        return out, i
    return body(0)[0]


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--definitions-repo", type=Path, required=True)
    args = ap.parse_args()
    repo = args.definitions_repo.resolve()
    dest = Path(__file__).resolve().parents[1] / "native/src/conversion/data"
    dest.mkdir(parents=True, exist_ok=True)
    def git(*argv):
        return subprocess.check_output(["git", "-C", str(repo), *argv])
    def read(commit, name):
        return git("show", f"{commit}:{name}").decode("utf-8-sig")
    def files(commit, folder):
        return sorted(git("ls-tree", "-r", "--name-only", commit, folder).decode().splitlines())
    for version, short in SNAPSHOTS.items():
        commit = git("rev-parse", short).decode().strip()
        metadata = json.loads(read(commit, "base/.ck3-version.json"))
        if metadata["version"] != version:
            raise ValueError(f"snapshot version mismatch: {version} vs {metadata['version']}")
        genes = {}
        for name in files(commit, "base/game/common/genes"):
            if not name.endswith(".txt"):
                continue
            text = read(commit, name)
            # These versioned vanilla files contain only non-stored special genes.
            # Some historical special-only files even omit the final outer brace.
            first = next((m.group() for m in TOKEN.finditer(text) if not m.group().startswith("#")), None)
            if first == "special_genes":
                continue
            try:
                parsed = parse(text)
            except ValueError as exc:
                raise ValueError(f"{version}: {name}: {exc}") from exc
            for cat, entries in parsed:
                if cat not in ("color_genes", "morph_genes", "accessory_genes") or not isinstance(entries, list):
                    continue
                for gene, fields in entries:
                    if gene is None or not isinstance(fields, list):
                        continue
                    templates = []
                    for template, values in fields:
                        if template is None or not isinstance(values, list):
                            continue
                        index = [v for k, v in values if k == "index"]
                        if index:
                            if len(index) != 1 or not isinstance(index[0], str) or not index[0].isdigit():
                                raise ValueError(f"invalid index: {name}:{gene}:{template}")
                            templates.append((template, int(index[0])))
                    genes.pop(gene, None)
                    genes[gene] = (cat, templates)
        if len(genes) < 50:
            raise ValueError(f"unexpected gene count for {version}: {len(genes)}")
        rows = [f"# Names and template indices only; version={version}; commit={commit}"]
        for gene, (cat, templates) in genes.items():
            rows.append(f"gene\t{cat}\t{gene}")
            rows.extend(f"template\t{index}\t{name}" for name, index in templates)
        payload = "\n".join(rows) + "\n"
        (dest / f"portraits-{version}.tsv").write_text(payload, encoding="utf-8", newline="\n")
        print(f"{version}: {len(genes)} genes, sha256={hashlib.sha256(payload.encode()).hexdigest()}")
        if version not in ("1.19.0.6", "1.20.0.3"):
            continue
        # Curated R25 lineage matches, not guesses from spelling similarity.
        definitions = {}
        for folder in ("men_at_arms_types", "modifiers", "artifacts/visuals"):
            keys = set()
            for name in files(commit, "base/game/common/" + folder):
                if name.endswith(".txt"):
                    keys.update(k for k, v in parse(read(commit, name)) if k is not None and isinstance(v, list))
            definitions[folder] = keys
        candidates = [
            ("regiment", "men_at_arms_types", "teutonic_knights", "order_serjeants", "00_holy_order_maa_types.txt; heavy infantry, damage/toughness 36/26 -> 36/24"),
            ("regiment", "men_at_arms_types", "chu_ko_nu", "shenbigong", "00_cultural_maa_innovations.txt; innovation_repeating_crossbow unlock"),
            ("artifact-type", "artifacts/visuals", "medium_animal_hide", "large_animal_hide", "00_court_artifacts.txt; wall_big slot"),
            ("modifier", "modifiers", "rice_fields_modifier", "terraced_hills_modifier", "fp2_yearly_events.txt:fp2_yearly.3000; same effect"),
        ]
        for new in sorted(definitions["modifiers"]):
            if re.fullmatch(r"artifact_[a-z_]+_scheme_phase_duration_add_\d+_modifier", new):
                candidates.append(("artifact-modifier", "modifiers", new.replace("_scheme_phase_duration_add_", "_scheme_power_add_"), new, "00_artifact_modifiers.txt; scheme rework, same scheme and tier"))
            if re.fullmatch(r"fp2_hawk_of_quraysh_\w+_modifier", new):
                candidates.append(("modifier", "modifiers", new.replace("_quraysh_", "_quarysh_"), new, "03_dlc_fp2_modifiers.txt; corrected spelling of same definition"))
        accepted = [(kind, old, new, proof) for kind, folder, old, new, proof in candidates
                    if old not in definitions[folder] and new in definitions[folder]]
        rows = [f"# R25 lineage facts from ck3-save-updater 0.5.33; independently checked target={version}; commit={commit}"]
        rows += ["\t".join(row) for row in accepted]
        (dest / f"content-{version}.tsv").write_text("\n".join(rows) + "\n", encoding="utf-8", newline="\n")
        print(f"  {len(accepted)} removed-content mappings verified against target definitions")


if __name__ == "__main__":
    main()
