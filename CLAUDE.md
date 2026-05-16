# search-memory

CLI Rust `search-memories` — recherche cross-projets / cross-sessions dans les fichiers de memoire Claude Code (`~/.claude/projects/*/memory/*.md`).

**Le consommateur, c'est moi (Claude).** L'outil existe pour me permettre de retrouver vite, avec peu de tokens, ce que j'ai deja appris sur les autres projets de Romain. Toute decision design doit favoriser cette utilite-la.

## Stack & build

- Rust edition 2021, binaire unique `search-memories`
- Dependencies : `glob`, `serde_json`, `chrono` (clock only). Pas de regex, pas de clap.
- Code : un seul fichier `src/main.rs` (~200 lignes).
- Build release : `cargo build --release`
- Install (PATH) : `cargo install --path . --force` -> `~/.cargo/bin/search-memories`

## Usage

```sh
search-memories "<terms...>"
```

- Tous les termes doivent etre presents (AND case-insensitive). Pas de fuzzy, pas d'OR.
- Output JSONL, un objet par ligne :
  ```json
  {"date":"YYYY/MM/DD","project":"agent-brain","file":"X.md","topic":"...","fragment":"..."}
  ```
- Aucun match -> stdout : `No matches found in memory sessions.`
- Tri : date de modification (mtime) descendante.

## Algo

1. Glob `$HOME/.claude/projects/*/memory/*.md`, exclut `MEMORY.md` (index).
2. Pour chaque fichier :
   - Lit, lowercase, cherche toutes les positions de chaque terme.
   - Si un terme manque -> skip.
   - **Sliding window** : calcule la fenetre [start, end] la plus petite contenant au moins une occurrence de chaque terme. O(n*k).
   - Anchor = centre de la fenetre.
   - Fragment = +/- 140 chars autour de l'anchor, whitespace collapse, ellipses si tronque.
3. Topic : parse YAML frontmatter, prend `description:` (prefere) ou `name:`, fallback premier H1, cap 120 chars.
4. Project : strip prefix `-Users-recarnot-(dev-|.|--)?` du nom de dossier Claude.
5. Sort hits par mtime desc, print JSONL.

## Choix design (et leurs pourquoi)

- **JSONL strict** plutot que pseudo-format : pipeable dans `jq`, parseable trivialement par moi.
- **5 fields** (date/project/file/topic/fragment) : chacun gagne sa place. `topic` permet de skip un faux positif sans Read le fichier ; `file` permet de Read si le fragment me titille.
- **Fenetre minimale** plutot qu'ancrage sur 1 terme : si une query a un terme generique (`pnpm`) et un terme specifique (`mcp:token`), je veux voir le contexte autour des DEUX, pas du seul generique.
- **MEMORY.md exclu** : c'est un index, pas du contenu — polluait sans topic.
- **Pas d'index/cache** : 191 fichiers scannes en 17ms a chaud. Inutile.

## Performance

- Cold start : ~340ms (OS cache vide).
- Hot : ~17ms sur 191 fichiers.
- Profile release avec `lto=thin`, `codegen-units=1`, `strip=true`.

## Itrations possibles si Romain le demande

- Ajuster `FRAGMENT_RADIUS` (140) si fragments trop courts/longs.
- Ajuster le cap topic (120 chars).
- Ajouter filtre par projet (`--project agent-brain`) — pour l'instant scope global volontairement.
- Ajouter filtre par date (`--since 2026-04-01`).

## Gerber

Ce projet est indexe dans **gerber** sous le slug `search-memory`.
Slug cross-projet : `caserne` (design system, conventions, preferences personnelles). Pour les sujets design/UI, conventions, stack : chercher aussi dans `caserne`.

Entites :
- **Notes** (atoms + documents) — memoire de connaissance, recherche semantique/fulltext
- **Tasks** — taches projet avec kanban 7 colonnes (inbox -> brainstorming -> specification -> plan -> implementation -> test -> done)
- **Issues** — problemes/bugs avec kanban 4 colonnes (inbox -> in_progress -> in_review -> closed)
- **Messages** — bus inter-sessions (context + reminder)

Skills disponibles :
- `/gerber:recall` — recherche contextuelle dans la memoire cross-projets
- `/gerber:capture` — capture rapide d'un atome de connaissance
- `/gerber:archive` — extraction et archivage fin de session
- `/gerber:session-complete` — cartographie de fin de session (.cave/ + archive)
- `/gerber:review` — maintenance hebdomadaire (notes, tasks, issues)
- `/gerber:import` — migration one-shot depuis .cave/
- `/gerber:inbox` — consulter les messages inter-sessions
- `/gerber:send` — envoyer un message inter-session
- `/gerber:task` — gestion des taches projet (kanban)
- `/gerber:issue` — gestion des issues projet
- `/gerber:rag` — recherche RAG dans le vault Gemini cross-projets 
- `/gerber:runbook` — composer le runbook d'un projet (run_cmd, url, env) depuis la stack du repo

## Contexte projet (.cave)

Le dossier `.cave/` contient la cartographie persistante du projet :
- `architecture.md` — vue d'ensemble, stack, flux de donnees
- `key-files.md` — fichiers critiques et leur role
- `patterns.md` — conventions et patterns recurrents
- `gotchas.md` — pieges, bugs resolus, workarounds

**Ne lis PAS ces fichiers au demarrage.** Lis-les a la demande, uniquement quand la question de l'utilisateur touche au domaine concerne (ex: question archi -> `architecture.md`, bug etrange -> `gotchas.md`). Pour une question triviale ou sans rapport avec le projet lui-meme, ne les lis pas du tout.
