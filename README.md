# DBGo

A fast, local-first SQL workbench. Open a SQLite database, browse tables, write
SQL, and move through rows without loading the whole table into memory.

![DBGo browsing a SQLite table](docs/dbgo-grid.png)

![DBGo SQL editor and results](docs/dbgo-sql.png)

![DBGo row detail panel](docs/dbgo-row.png)

![DBGo table designer](docs/dbgo-designer.png)

What's here so far:

- Tauri 2 native desktop window (no browser app, no server)
- Rust core with SQLite (WAL, memory-mapped I/O)
- Create a new SQLite database from scratch
- Open an existing SQLite file with the native file picker
- Visual table designer: types, PK/NN/unique, defaults, foreign keys, FK indexes
- Live DDL preview; every schema change is recorded in hidden `_dbgo_migrations`
- List tables in the sidebar
- Browse rows with a virtualized grid
- Keyset pagination (`rowid`) — fast on large tables
- Row count capped at 1,000,000 to avoid expensive full counts
- NULL / integer / real / text / BLOB cell rendering
- SQL editor (CodeMirror 6) with schema-aware autocomplete and `Ctrl/Cmd+Enter`
- Query history and read-only query results in the same grid
- Click-to-sort columns, server-side filtering, and a row detail panel
- Interactive ERD diagram: foreign-key edges, auto-layout, search, drag positions
- `dbgo` CLI with human, JSON, and JSONL output, plus `dbgo init`
- Tailwind CSS v4 UI

## Requirements

- Rust 1.85+ (tested with 1.99)
- Node.js 20+ and pnpm
- Linux: `webkit2gtk-4.1`, `gtk+-3.0`, `libsoup-3.0`, `javascriptcoregtk-4.1`

## Run

```bash
pnpm install
pnpm tauri:dev
```

Open a database directly from the command line (after a build):

```bash
pnpm tauri:build --no-bundle
./target/release/db-go samples/companies.db
```

During development, run `pnpm tauri:dev` and use the Open database button.

Build a release bundle:

```bash
pnpm tauri:build
```

## CLI

Install the `dbgo` command:

```bash
cargo install --path crates/cli
```

That puts `dbgo` in `~/.cargo/bin`, which is already on `PATH` for Rust users.
Then, from anywhere:

```bash
dbgo init ./myapp.db
dbgo tables  ./samples/companies.db
dbgo schema  ./samples/companies.db companies
dbgo count   ./samples/companies.db companies
dbgo page    ./samples/companies.db companies --limit 50 --cursor 201
dbgo query   ./samples/companies.db "select id, name, city from companies limit 20"

# machine-readable
dbgo --json  tables ./samples/companies.db
dbgo --jsonl query  ./samples/companies.db "select * from companies limit 100"
```

`dbgo init` creates a new empty database. Add `--force` to replace an existing
file; a timestamped backup is kept.

## Create a database

- GUI: click **Create database**, choose a path, then design your first table.
- CLI: `dbgo init ./myapp.db` (use `--force` to replace with a backup).
- The table designer supports primary keys, NOT NULL, UNIQUE, defaults,
  foreign keys, ON DELETE actions, and indexes on foreign keys.
- A live SQL preview shows exactly what will run. Every schema change is
  recorded in the hidden `_dbgo_migrations` table for future D1 export.

`query` is read-only. `page` uses keyset pagination on `rowid`. Run
`dbgo <command> --help` for details.

Without installing:

```bash
cargo run -p db-go-cli -- tables ./samples/companies.db
```

## SQL editor

Open a database, select a table, and switch to the **SQL** tab. The editor is
prefilled with a `SELECT`, and `Ctrl/Cmd+Enter` runs the current statement.
Results appear in the same virtualized grid. Queries are **read-only** for now;
writes will arrive with audit logs and automatic backups.

## Sort, filter, inspect

- Click a column header to cycle ascending → descending → unsorted.
- Use the filter bar for `contains`, comparisons, or null checks.
- Click any row to open the detail panel and copy it as JSON or CSV.

## Diagram

Switch to the **Diagram** tab to see every table and its foreign-key
relationships.

![DBGo schema diagram](docs/dbgo-diagram.png)

- Columns show `PK` / `FK` badges and types.
- Drag tables to rearrange; positions persist per database.
- **Auto layout** re-runs Dagre; **Fit view** zooms to the whole schema.
- Search highlights matching tables.
- Double-click a table to open its rows.

## Sample data

Generate a 10,000-company / 30,000-link SQLite database for testing:

```bash
python3 scripts/make_sample.py
```

Then open `samples/companies.db` in DBGo. The sample exercises NULLs, integers,
reals, text, BLOBs, and a table larger than one page.

## Verification

```bash
cargo test -p db-go-core
cargo clippy --workspace --all-targets -- -D warnings
pnpm build
pnpm tauri:build --no-bundle
```

## Layout

```
crates/core/     SQLite engine, schema introspection, paging
crates/cli/      dbgo command-line interface
src-tauri/       Tauri commands and app state
src/             React UI (Vite)
```

## Next

- Insert / update / delete with audit + undo
- Postgres and MySQL drivers
- MCP mode (`dbgo mcp`)
- D1 export and deploy

## License

MIT
