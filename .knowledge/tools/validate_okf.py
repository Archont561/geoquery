#!/usr/bin/env python3
"""validate_okf.py — OKF v0.2 conformance validator for this knowledge bundle.

Zero-dependency (stdlib only). Implements the conformance rules of the
Open Knowledge Format specification, version 0.2
(https://github.com/GoogleCloudPlatform/knowledge-catalog, okf/SPEC.md):

  1. Every non-reserved .md file has a parseable YAML frontmatter block.
  2. Every frontmatter block has a non-empty `type` field.
  3. Reserved files follow their roles:
     - index.md  : no frontmatter, except bundle-root may declare `okf_version`
                   only; body is sectioned `* [Title](url) - description` lists.
     - log.md    : flat date-grouped entries, newest first, ISO YYYY-MM-DD.

Checks beyond hard conformance are reported as warnings (status vocabulary,
ISO 8601 timestamps, actor convention, missing description, broken links,
past stale_after). Usage:

    python3 .knowledge/tools/validate_okf.py [BUNDLE_DIR]

Exit code 0 = conformant (warnings allowed), 1 = non-conformant.
"""
import os
import re
import sys

ISO_DT = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(Z|[+-]\d{2}:\d{2})$")
ISO_DATE_HEADING = re.compile(r"^## (\d{4}-\d{2}-\d{2})\s*$")
ACTOR = re.compile(r"^(human:[^\s]+|process:[^\s]+|[A-Za-z0-9_.-]+/[^\s]+)$")
RESERVED = {"index.md", "log.md"}
STATUS_VOCAB = {"draft", "stable", "deprecated"}


def parse_frontmatter(text):
    """Return (fields_dict, error). Tries PyYAML, falls back to a line parser
    sufficient for flat 'key: value' frontmatter with flow lists/maps."""
    if not text.startswith("---"):
        return {}, "file does not start with a '---' frontmatter delimiter"
    try:
        end = text.index("\n---", 3)
    except ValueError:
        return {}, "frontmatter block is not closed with '---'"
    raw = text[3:end]
    try:
        import yaml  # type: ignore
        fields = yaml.safe_load(raw)
        if not isinstance(fields, dict):
            return {}, "frontmatter did not parse to a mapping"
        return fields, None
    except ImportError:
        pass
    fields = {}
    for line in raw.splitlines():
        if not line.strip() or line.strip().startswith("#"):
            continue
        key, sep, val = line.partition(":")
        if not sep:
            return fields, f"unparseable frontmatter line: {line!r}"
        key, val = key.strip(), val.strip()
        fields[key] = _parse_flow(val)
    return fields, None


def _parse_flow(val):
    """Parse a YAML flow scalar / [list] / {map} without PyYAML."""
    if val.startswith("{") and val.endswith("}"):
        out = {}
        for pair in _split_top(val[1:-1], ","):
            k, _, v = pair.partition(":")
            out[k.strip()] = _parse_flow(v.strip())
        return out
    if val.startswith("[") and val.endswith("]"):
        return [_parse_flow(i.strip()) for i in _split_top(val[1:-1], ",") if i.strip()]
    if len(val) >= 2 and val[0] == val[-1] and val[0] in "\"'":
        return val[1:-1]
    return val


def _split_top(s, sep):
    """Split on sep, ignoring separators inside quotes."""
    parts, buf, quote = [], "", None
    for ch in s:
        if quote:
            buf += ch
            if ch == quote:
                quote = None
        elif ch in "\"'":
            quote = ch
            buf += ch
        elif ch == sep:
            parts.append(buf)
            buf = ""
        else:
            buf += ch
    parts.append(buf)
    return parts


def split_body(text):
    end = text.index("\n---", 3)
    return text[end + 4:]


def check_links(bundle, path, body, warn):
    for m in re.finditer(r"\]\(([^)\s]+)\)", body):
        target = m.group(1)
        if target.startswith(("http://", "https://", "mailto:", "#")):
            continue
        target = target.split("#", 1)[0]
        if not target:
            continue
        if target.startswith("/"):
            fs = os.path.join(bundle, target.lstrip("/"))
        else:
            fs = os.path.normpath(os.path.join(os.path.dirname(path), target))
        if not os.path.exists(fs):
            warn(path, f"broken link to '{target}' (consumers MUST tolerate, but it may be a typo)")


def main():
    bundle = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "..")
    bundle = os.path.normpath(bundle)
    errors, warnings = [], []

    def err(path, msg):
        errors.append(f"{os.path.relpath(path, bundle)}: {msg}")

    def warn(path, msg):
        warnings.append(f"{os.path.relpath(path, bundle)}: {msg}")

    md_files, indexes, logs = [], [], []
    for root, _dirs, files in os.walk(bundle):
        for fn in sorted(files):
            p = os.path.join(root, fn)
            rel = os.path.relpath(p, bundle)
            if fn.endswith(".md"):
                md_files.append(p)
            if fn == "index.md":
                indexes.append(p)
            if fn == "log.md":
                logs.append(p)
            if fn in ("INDEX.md", "Index.md", "LOG.md"):
                err(p, "reserved filename must be lowercase 'index.md'/'log.md'")

    n_concepts = 0
    for p in sorted(md_files):
        rel = os.path.relpath(p, bundle)
        fn = os.path.basename(p)
        text = open(p, encoding="utf-8").read()
        if fn in RESERVED:
            if fn == "index.md":
                is_root = os.path.dirname(p) == bundle
                if text.startswith("---"):
                    fields, e = parse_frontmatter(text)
                    if e:
                        err(p, f"index frontmatter: {e}")
                    else:
                        extra = {k: v for k, v in fields.items() if k != "okf_version"}
                        if extra or not is_root:
                            err(p, "index.md must have no frontmatter "
                                   "(only bundle-root may declare okf_version)")
                body = text.split("---", 2)[2] if text.startswith("---") else text
                if not re.search(r"^# ", body, re.M):
                    warn(p, "index body has no section headings")
                if not re.search(r"^\* \[.+\]\(.+\)", body, re.M):
                    warn(p, "index body has no '* [Title](url) - description' entries")
                check_links(bundle, p, body, warn)
            else:  # log.md
                if text.startswith("---"):
                    err(p, "log.md must not carry frontmatter")
                dates = ISO_DATE_HEADING.findall(text)
                for line in text.splitlines():
                    if re.match(r"^## \d", line) and not ISO_DATE_HEADING.match(line):
                        err(p, f"log date heading not ISO YYYY-MM-DD: {line!r}")
                if dates != sorted(dates, reverse=True):
                    err(p, "log entries are not newest-first")
            continue

        # ---- concept document ----
        n_concepts += 1
        fields, e = parse_frontmatter(text)
        if e:
            err(p, e)
            continue
        if not fields.get("type", "") or not str(fields.get("type", "")).strip():
            err(p, "missing required non-empty 'type' field")
        if not str(fields.get("description", "")).strip():
            warn(p, "no 'description' (recommended)")
        status = str(fields.get("status", "stable"))
        if status not in STATUS_VOCAB:
            warn(p, f"status '{status}' outside OKF vocabulary {sorted(STATUS_VOCAB)}")
        for key in ("created", "updated", "stale_after"):
            if key in fields and not ISO_DT.match(str(fields[key])):
                warn(p, f"{key}='{fields[key]}' is not ISO 8601 with explicit UTC offset")
        gen = fields.get("generated")
        if gen is not None:
            by = gen.get("by") if isinstance(gen, dict) else None
            if not by or not ACTOR.match(str(by)):
                warn(p, "generated.by missing or not in actor convention "
                        "(<producer>/<version>, human:<id>, process:<id>)")
            at = gen.get("at") if isinstance(gen, dict) else None
            if at and not ISO_DT.match(str(at)):
                warn(p, f"generated.at='{at}' is not ISO 8601 with explicit UTC offset")
        ver = fields.get("verified")
        if ver is not None:
            ver = ver if isinstance(ver, list) else [ver]
            for v in ver:
                if not (isinstance(v, dict) and ACTOR.match(str(v.get("by", "")))):
                    warn(p, "verified entry missing actor-convention 'by'")
                    break
        if "stale_after" in fields and ISO_DT.match(str(fields["stale_after"])):
            import datetime
            sa = str(fields["stale_after"]).replace("Z", "+00:00")
            if datetime.datetime.fromisoformat(sa) <= datetime.datetime.now(datetime.timezone.utc):
                warn(p, f"stale_after is in the past (concept is stale)")
        check_links(bundle, p, split_body(text), warn)

    # summary
    print(f"OKF v0.2 validation: {bundle}")
    print(f"  concepts: {n_concepts}   index.md files: {len(indexes)}   log.md files: {len(logs)}")
    for w in warnings:
        print(f"  WARN  {w}")
    for e in errors:
        print(f"  ERROR {e}")
    if errors:
        print(f"RESULT: NON-CONFORMANT ({len(errors)} error(s), {len(warnings)} warning(s))")
        return 1
    print(f"RESULT: CONFORMANT with OKF v0.2 ({len(warnings)} warning(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
