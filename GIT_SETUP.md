# Git Setup Notes

Project moved from another machine and was never turned into a git repo here (`git status` currently fails — no `.git` at root). Before running `git init`, be aware of two things in this tree:

## 1. Nested repo: `obsidian-vault-template/`

`obsidian-vault-template/` has its own `.git`, pointing at:

```
origin  git@github.com:voidashi/obsidian-vault-template.git
```

If you `git init` at root and `git add -A`, git will treat this as an embedded repository (gitlink), not track its file contents — `git status` will show it as a weird untracked/modified entry, and a plain clone of the parent repo will leave that folder empty.

Options:
- **Recommended:** add `obsidian-vault-template/` to root `.gitignore` (it's an upstream reference clone, not part of the app — the actual compiled copy lives in `src-tauri/vault-template/`, which is a plain directory with no nested `.git`).
- Or register it as a proper submodule (`git submodule add`) if you want the parent repo to pin a specific commit of it.
- Or `rm -rf obsidian-vault-template/.git` if you don't need it to stay an independent clone (only do this if you're sure — it removes its own history/remote link, not the files).

## 2. `src-tauri/target/` — ~19 GB build cache

Already covered by `src-tauri/.gitignore` (`/target/`), so it won't get staged. Just don't `git add -f` anything under it. Same for root `node_modules/` (covered by root `.gitignore`).

## Suggested first commit

```bash
# from repo root
echo "obsidian-vault-template/" >> .gitignore   # if keeping option 1 above
git init
git add -A
git status   # sanity check: no target/, no node_modules/, no gitlink for obsidian-vault-template
git commit -m "Initial commit"
```
