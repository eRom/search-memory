# search-memories

CLI minimal pour fouiller les fichiers de memoire Claude Code cross-projets / cross-sessions.

Scope : exclusivement `~/.claude/projects/*/memory/*.md`. Pas d'index, pas de cache.

## Install

```sh
cargo install --path .
```

Le binaire `search-memories` est installe dans `~/.cargo/bin/`.

## Usage

```sh
search-memories "<query>"
```

- Tous les termes doivent etre presents dans le fichier (AND, case-insensitive).
- Tri par date de modification descendante.
- Output JSONL (un objet par ligne) : `{"date":"YYYY/MM/DD","fragment":"..."}`.
- Aucun match -> `No matches found in memory sessions.`

## Exemples

```sh
search-memories "FTS5"
search-memories "OAuth romain"
search-memories "deploy vps"
```

Pipe dans `jq` :

```sh
search-memories "OAuth" | jq -r '"\(.date)  \(.fragment)"'
```

## Performance

~191 fichiers markdown scannes en <20ms (cold ~340ms premiere exec).
