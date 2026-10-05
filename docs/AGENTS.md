# Docs guide for agents

These rules keep the docs readable for people and accurate over time. They
apply to every Markdown file in the repository, not only this folder.

## Write for people first

- Use plain technical English: short sentences, active voice, common words,
  and "you" for the reader.
- Start each page with one paragraph that says what the page covers and who
  it is for.
- Define a term the first time you use it, or link to the glossary in
  [ARCHITECTURE.md](../ARCHITECTURE.md#glossary).
- In pages for people, don't use internal shorthand such as "seam",
  "posture", "owning layer", "provenance", "attended", or "non-wedging". Say
  what you mean instead. Agent guides (`AGENTS.md` files) may use "owning
  layer" and "provenance" as defined rule terms.
- Prefer a table or a list over a long paragraph. Prefer one example over
  three sentences of description.
- Use Mermaid for diagrams. Don't add image-only diagrams, because they drift
  from their source. The one exception is the README banner in
  `docs/assets`, which `docs/assets/generate.mjs` generates. Edit the script
  and regenerate; never hand-edit the banner SVG files.

## One fact, one home

Every fact lives on exactly one page. Other pages link to it instead of
repeating it.

| Fact | Its home |
| --- | --- |
| Whether a capability works on a platform | [reference/capabilities.md](reference/capabilities.md) |
| Commands that validate a change | [reference/testing.md](reference/testing.md) |
| First-run setup steps | [getting-started.md](getting-started.md) |
| The Servo version, lockfiles, and patches | [reference/dependencies.md](reference/dependencies.md) |
| Crate purposes and dependency rules | [reference/crates.md](reference/crates.md) |
| Principles and layers | [ARCHITECTURE.md](../ARCHITECTURE.md) |
| The command envelope and fallbacks | [concepts/controller.md](concepts/controller.md) |
| Contribution and approval process | [CONTRIBUTING.md](../CONTRIBUTING.md) |
| Rules for agents | [AGENTS.md](../AGENTS.md) and the area guides |

When you change a fact, change its home page, then check for stale copies:

```sh
git grep -n "<the old fact>" -- '*.md'
```

## Only write what is true

- Before you document a command, script, Gradle task, file, type, or
  function, confirm it exists, for example with `git grep`. Run the command
  when you can.
- Describe what the code does today. If the code breaks a principle, describe
  the actual behavior and raise the gap in an issue. Don't describe the
  intended behavior as if it were real.
- A test page that loads is not evidence that a capability is supported.
- Public docs describe validated architecture and APIs. Plans, status
  reports, investigation notes, and pull request evidence belong in issues and
  pull requests.

## Page layout

```text
docs/
  README.md            Docs home: the map of every page
  getting-started.md   First run
  concepts/            How ExplorerKit works, one idea per page
  platforms/           One guide per platform
  reference/           Facts to look up: capabilities, crates, testing, dependencies
```

This layout matches the content tree of [Nimbus](https://github.com/cloudflare/nimbus),
so the folder can become a docs site later without moving pages. To stay
compatible:

- Give each page exactly one `#` heading, its title.
- Use relative links between pages.
- Use GitHub-flavored Markdown, and avoid raw HTML outside the root README.
- Keep heading text stable. Other pages link to headings by their anchor.

## When you add, move, or delete a page

1. Update [docs/README.md](README.md).
2. Update [llms.txt](../llms.txt) with the page's path and a one-line summary.
3. Update the docs map in [AGENTS.md](../AGENTS.md) if the page answers a
   question listed there.
4. Find and fix every link to the old path:

   ```sh
   git grep -n "old-page.md"
   ```

## Check your change

```sh
git diff --check
python3 - <<'EOF'
import pathlib, re, sys
broken = []
for page in pathlib.Path('.').rglob('*.md'):
    if any(part in {'node_modules', 'vendor', 'upstream', 'patches', 'target'} for part in page.parts):
        continue
    for target in re.findall(r'\]\(([^)\s#]+)(?:#[^)]*)?\)', page.read_text(encoding='utf-8')):
        if '://' in target or target.startswith('mailto:'):
            continue
        if not (page.parent / target).exists():
            broken.append(f'{page}: {target}')
print('\n'.join(broken) or 'no broken relative links')
sys.exit(1 if broken else 0)
EOF
```
