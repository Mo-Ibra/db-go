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
- Create a database project: `db.sqlite`, `AGENTS.md`, `schema.md`, `migrations/`, `backups/`
- Open an existing SQLite file; DBGo creates agent-ready files beside it
- Recent databases on the welcome screen, with missing-file detection
- Visual table designer: types, PK/NN/unique, defaults, foreign keys, FK indexes
- Structure inspector: DDL, columns, indexes, foreign keys, and index management
- Alter tables from the GUI or CLI: add, rename, and drop columns with SQL preview
- Drop/rename tables, manage triggers, and run VACUUM/ANALYZE/PRAGMA
- Arbitrary SQL execution with `dbgo exec`, explicit write guard, transactions, and RETURNING
- AND/OR/NOT filter expressions for updates and deletes
- Indexes, views, and numbered migration files
- Every schema change gets an audit row and a migration record
- List tables in the sidebar
- Browse rows with a virtualized grid
- Keyset pagination (`rowid`) — fast on large tables
- Row count capped at 1,000,000 to avoid expensive full counts
- NULL / integer / real / text / BLOB cell rendering
- SQL editor (CodeMirror 6) with schema-aware autocomplete and `Ctrl/Cmd+Enter`
- Query plan explorer with full-scan and index warnings
- Clickable foreign-key navigation from row details
- Table/index size stats and CSV/JSON result export
- CSV/JSONL import with preview, column mapping, and create-table-from-file
- Cloudflare D1 export bundle (CLI + GUI): schema, batched data, Wrangler config, README
- Query history and read-only query results in the same grid
- Click-to-sort columns, server-side filtering, and a row detail panel
- Interactive ERD diagram: foreign-key edges, auto-layout, search, drag positions
- GUI data editing with staged changes and one explicit **Save changes**
- Inline cell editing, multi-row selection, and TSV copy/paste
- Backup browser with manual snapshots and restore
- CLI writes: insert, upsert, update, delete, with audit sessions
- Live refresh when the CLI changes the open database
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

`cargo install` copies a compiled binary — it does **not** update automatically
when you edit the source. After changing `crates/cli` or `crates/core`, reinstall:

```bash
cargo install --path crates/cli --force
```

For a fast development loop, run from the workspace instead:

```bash
cargo run -p db-go-cli -- tables ./samples/companies.db
```

Or build the debug binary and run it directly:

```bash
cargo build -p db-go-cli
./target/debug/dbgo tables ./samples/companies.db
```

Then, from anywhere:

```bash
dbgo init ./database              # project folder
dbgo init ./app.db --single-file  # bare SQLite file

dbgo tables  ./samples/companies.db
dbgo schema  ./samples/companies.db companies
dbgo count   ./samples/companies.db companies
dbgo page    ./samples/companies.db companies --limit 50 --cursor 201
dbgo query   ./samples/companies.db "select id, name, city from companies limit 20"
dbgo exec    ./samples/companies.db "select count(*) from companies"
dbgo explain ./samples/companies.db "select * from companies where domain = 'acme.test'"
dbgo stats   ./samples/companies.db
dbgo export  ./samples/companies.db companies --format csv --output companies.csv
dbgo export  ./samples/companies.db --query "select * from companies" --format json --output companies.json
dbgo import  ./database people --file people.csv --preview
dbgo d1 export ./database --output ./d1 --name my-db

# project commands (run inside the project folder)
cd database
dbgo graph . --json
dbgo create-table . --file table.json
dbgo alter-table . companies --add-column '{"name":"country","type_name":"TEXT","primary_key":false,"not_null":false,"unique":false,"default_value":null,"references":null}'
dbgo alter-table . companies --rename-column city=location
dbgo alter-table . companies --drop-column legacy_id --yes
dbgo drop-table . legacy_companies --yes
dbgo rename-table . companies organizations
dbgo trigger create . audit_companies --sql "create trigger audit_companies after update on companies begin select 1; end"
dbgo trigger list . companies
dbgo trigger drop . audit_companies --yes
dbgo vacuum .
dbgo analyze .
dbgo pragma . user_version
dbgo index create . companies --name idx_companies_city --columns city
dbgo index list . companies
dbgo view create . active_companies --sql "select * from companies where active = 1"
dbgo migrations .
dbgo migrate .
dbgo insert . companies --file rows.jsonl
dbgo upsert . companies --key domain --file rows.jsonl
dbgo update . companies --where '{"column":"id","op":"eq","value":1}' --set '{"city":"Berlin"}' --yes
dbgo delete . companies --where '{"column":"id","op":"eq","value":1}' --yes
dbgo sessions .
dbgo backup .
dbgo import . people --file people.csv
dbgo import . --file people.csv --create-table
dbgo backups .
dbgo restore . --latest --yes
# or restore one specific snapshot:
dbgo restore . --backup backups/db.sqlite.bak-1794310717376 --yes

# machine-readable
dbgo --json  tables ./samples/companies.db
dbgo --jsonl query  ./samples/companies.db "select * from companies limit 100"
```

`dbgo init ./database` creates a project folder with the database, an agent
guide, a schema snapshot, migrations, and backups. Add `--force` to replace an
existing project; a timestamped backup is kept.

## Create a database project

- GUI: click **Create database**, choose a folder, then design your first table.
- CLI: `dbgo init ./database` (or `dbgo init ./app.db --single-file` for a bare DB).
- The table designer supports primary keys, NOT NULL, UNIQUE, defaults,
  foreign keys, ON DELETE actions, and indexes on foreign keys.
- A live SQL preview shows exactly what will run. Every schema change is
  recorded in the hidden `_dbgo_migrations` table and in `migrations/`.

A project looks like this:

```
database/
├── db.sqlite
├── AGENTS.md        # instructions for coding agents
├── schema.md        # generated schema snapshot
├── dbgo.json        # project config
├── migrations/      # numbered DDL, ready for D1
└── backups/         # timestamped snapshots
```

The agent should work from that folder and use `.` as the database path:

```bash
dbgo graph . --json
dbgo create-table . --file table.json
dbgo backup .
```

`query` is read-only. `page` uses keyset pagination on `rowid`. Run
`dbgo <command> --help` for details.

## Open an existing database

Opening a bare SQLite file in the app converts its folder into an agent-ready
project, including:

```
companies.db
AGENTS.md          # CLI commands and project rules for agents
schema.md          # generated schema snapshot
dbgo.json          # project config pointing at companies.db
migrations/        # existing migration records materialized as .sql files
backups/           # future backups go here
```

- Existing `AGENTS.md` and `schema.md` files are preserved. DBGo appends a
  marked section instead of overwriting user content.
- Existing migration records in `_dbgo_migrations` are written to
  `migrations/` if the files are missing.
- Future backups go into `backups/`, and schema changes write numbered
  migration files.

The same adoption is available from the CLI:

```bash
dbgo agent-guide ./path/to/companies.db
```

## Recent databases

The welcome screen keeps your last 10 opened databases.

- Click a recent entry to reopen it immediately.
- Entries whose files no longer exist are marked **missing** and cannot be opened.
- Click **×** to remove one entry, or **Clear** to remove all entries.
- Recent paths are stored locally on this machine; nothing is uploaded.
- Opening a database from the picker, the welcome screen, or a command-line argument
  updates the list.

## Schema changes

The **Structure** tab shows the table DDL, columns, indexes, foreign keys, and
lets you create or drop indexes. **Alter table** stages add/rename/drop column
operations with a live SQL preview, then applies them as one migration.

```bash
dbgo alter-table . companies --add-column '{"name":"country","type_name":"TEXT","primary_key":false,"not_null":false,"unique":false,"default_value":null,"references":null}'
dbgo alter-table . companies --rename-column city=location
dbgo alter-table . companies --drop-column legacy_id --yes
dbgo alter-table . companies --file alter.json --sql-only
```

- Dropping a column requires `--yes`.
- Added `NOT NULL` columns need a default.
- Added columns cannot be primary keys or `UNIQUE`; create a unique index instead.
- A column used by an index must have its index dropped first.
- `--sql-only` validates and prints the SQL without touching the database.

Indexes and views:

```bash
dbgo index create . companies --name idx_companies_city --columns city
dbgo index create . companies --name idx_companies_domain --columns domain --unique
dbgo index list . companies
dbgo index drop . companies idx_companies_city --yes

dbgo view create . active_companies --sql "select * from companies where active = 1"
dbgo view list .
dbgo view drop . active_companies --yes
```

### DDL and maintenance

```bash
dbgo drop-table . legacy_companies --yes
dbgo rename-table . companies organizations

dbgo trigger create . audit_companies --sql "create trigger audit_companies after update on companies begin select 1; end"
dbgo trigger create . audit_companies --file trigger.sql
dbgo trigger list . companies
dbgo trigger drop . audit_companies --yes

dbgo vacuum .
dbgo analyze .
dbgo pragma . user_version
dbgo pragma . user_version=3 --yes
```

- `drop-table` and `trigger drop` require `--yes`.
- Schema changes write a migration file and refresh `schema.md`.
- `vacuum`, `analyze`, and write PRAGMAs are audited but do not create migrations.
- Changing a column type or constraint is not a dedicated command. Use
  `dbgo exec` or a migration file with a table rebuild.
- `ATTACH` and explicit transaction control are available through
  `dbgo exec --no-transaction`.

### Migrations

Every schema change writes a numbered file into `migrations/` and records it in
the hidden `_dbgo_migrations` table. Apply files added by hand or by another
tool with:

```bash
dbgo migrations .   # applied and pending files
dbgo migrate .      # apply pending files in filename order
```

Each applied migration gets an audit session. Do not edit a migration that is
already recorded as applied. Migrations should contain plain SQL statements,
not `BEGIN`/`COMMIT`.

### Backups and restore

Backups are **manual**. DBGo never creates one automatically for normal data or
schema changes.

```bash
dbgo backup .                               # take a snapshot now
dbgo backups .                              # list snapshots, newest first
dbgo restore . --latest --yes               # restore the newest snapshot
dbgo restore . --backup <path> --yes        # restore one snapshot
```

- The app's **Backups** dialog has a **Take backup** button.
- The dialog lists snapshots with size and timestamp.
- A restore validates the snapshot before applying it.
- Take a backup before restoring if you need to undo the restore.
- The restore itself is recorded in `_dbgo_audit`.
- Backups live in `backups/` for project folders, next to the file for
  single-file databases.

## Writing data

Every write is transactional and records an audit row. Backups are **not**
created automatically; use `dbgo backup .` or the **Take backup** button when
you want a snapshot.

```bash
dbgo insert . companies --file rows.jsonl
dbgo upsert . companies --key domain --file rows.jsonl
dbgo update . companies --where '{"column":"id","op":"eq","value":1}' --set '{"city":"Berlin"}' --yes
dbgo delete . companies --where '{"column":"id","op":"eq","value":1}' --yes
dbgo sessions .
```

- `rows.jsonl` contains one JSON object per line.
- `upsert` needs a `UNIQUE` or primary-key constraint on the key column.
- `update` / `delete` require `--yes`.
- Primary key columns cannot be updated.
- `dbgo sessions .` lists recent write sessions and action counts.

### Filter expressions

`--where` accepts either the original single condition or an AND/OR/NOT
expression tree:

```json
{
  "type": "and",
  "filters": [
    { "type": "condition", "column": "id", "op": "gte", "value": "2" },
    {
      "type": "or",
      "filters": [
        { "type": "condition", "column": "city", "op": "eq", "value": "Berlin" },
        { "type": "condition", "column": "city", "op": "eq", "value": "Paris" }
      ]
    }
  ]
}
```

```bash
dbgo update . companies \
  --where '{"type":"not","filter":{"type":"condition","column":"id","op":"eq","value":1}}' \
  --set '{"city":"Unknown"}' \
  --yes
```

- Node types: `condition`, `and`, `or`, `not`.
- Conditions use the same operators: `contains`, `eq`, `neq`, `gt`, `gte`,
  `lt`, `lte`, `is_null`, `not_null`.
- Old `{"column":...,"op":...,"value":...}` filters still work.
- For arbitrary SQL expressions, use `dbgo exec`.

The desktop app polls `PRAGMA data_version`. When the CLI changes the database,
the open table, schema, and diagram refresh automatically.

Existing projects, and bare databases opened with `agent-guide`, can refresh
their agent files with:

```bash
dbgo agent-guide .
dbgo agent-guide ./path/to/companies.db
```

## Editing data in the app

Row edits are **staged**, not written cell by cell. The grid highlights pending
rows and the **Save changes** bar shows how many rows are new, edited, or
deleted.

- **+ Row** opens the row editor and adds a pending insert.
- **Double-click a cell** (or press `Enter`/`F2`) to edit inline.
  - `Enter` stages the cell, `Tab` stages and moves, `Esc` cancels.
  - Type `<NULL>` to set a nullable cell to NULL.
- The row detail panel has **Edit row** and **Delete row** actions.
- Use row checkboxes to select multiple rows; `Shift` extends the range.
- **Copy TSV** copies selected rows with a header row. `Ctrl/Cmd+C` works from
  the grid, and `Ctrl/Cmd+V` pastes TSV blocks starting at the active cell.
- **Delete** stages selected rows for deletion; **Clear** only clears selection.
- **Save changes** commits every staged change in one transaction, with one
  audit session. It does not create a backup automatically.
- **Discard** throws away all staged changes without touching the database.
- Navigation is blocked while changes are pending: switching tables, filtering,
  sorting, paging, refreshing, opening another database, and creating a table
  wait until you save or discard.
- If the CLI writes while edits are staged, DBGo keeps your changes and shows a
  notice. Saving or discarding then reloads the fresh data.

The GUI SQL editor remains read-only. Use the CLI for arbitrary SQL writes.

Without installing:

```bash
cargo run -p db-go-cli -- tables ./samples/companies.db
```

## SQL editor

Open a database, select a table, and switch to the **SQL** tab. The editor is
prefilled with a `SELECT`, and `Ctrl/Cmd+Enter` runs the current statement.
Results appear in the same virtualized grid. The GUI SQL editor is **read-only**;
use the staging grid or the CLI for writes.

Click **Explain** to see SQLite's query plan without running the statement:

```bash
dbgo explain . "select * from companies where domain = 'acme.test'"
dbgo --json explain . "select ..."
```

The plan panel highlights:
- `⚠ SCAN table` — a full table scan
- `↗ SEARCH ... USING INDEX` / `USING COVERING INDEX` — an index lookup
- `USE TEMP B-TREE` — a sort that may need attention

## Execute arbitrary SQL

`dbgo exec` is read-only unless you explicitly allow writes.

```bash
# read-only
dbgo exec . "select * from companies limit 20"

# writes and DDL require --write and --yes
dbgo exec . "update employees set salary = salary + 1" --write --yes
dbgo exec . "insert into archive select * from employees" --write --yes
dbgo exec . --file script.sql --write --yes

# return rows from a single write statement
dbgo exec . "update companies set city = 'Berlin' returning id, city" \
  --write --yes --returning

# maintenance / statements that cannot run inside a transaction
dbgo exec . "vacuum" --write --yes --no-transaction
dbgo exec . "attach database 'other.db' as other; insert into archive select * from other.t;" \
  --write --yes --no-transaction
```

- Read-only by default; writes require `--write` and `--yes`.
- Normal writes are wrapped in one transaction.
- `--no-transaction` runs the SQL directly, for `VACUUM`, `ATTACH`, write
  PRAGMAs, or scripts with their own `BEGIN`/`COMMIT`.
- `--returning` executes a single statement and returns rows; without it,
  multi-statement writes report the total affected rows.
- Writes get one audit session and no automatic backup.
- `--limit` caps returned rows.

## Foreign-key navigation

In the data view, click a row to open its details. FK values are links:

- Clicking one opens the referenced table filtered to that exact key.
- The filter bar shows the column and value used.
- The Structure tab also lists foreign keys with an **Open** action for the
  referenced table.

## Size stats

The **Structure** tab has a Storage card showing:

- row count
- table size
- index size
- index count

```bash
dbgo stats . 
dbgo --json stats .
```

- Uses SQLite's `dbstat` table for page sizes when available.
- Uses `sqlite_stat1` row estimates when present, otherwise a capped row count.
- Per-index sizes appear under the index list.

## Export

Export the current table view with its active filter and sort, or export a SQL
query:

```bash
dbgo export . companies --format csv --output companies.csv
dbgo export . companies --format json --where '{"column":"id","op":"gte","value":1}'
dbgo export . --query "select * from companies order by id" --format json --output companies.json
```

- The Data view has a CSV/JSON picker and **Export** button.
- The SQL view exports the current query without applying the Max rows limit.
- Exports are read-only and do not create backups.
- BLOB values are base64-encoded.
- Omitting `--output` writes data to stdout and the row count to stderr.

## Import

The header **Import** button opens a preview-first importer.

- Choose CSV, JSON, or JSONL.
- CSV supports headers/no headers and comma, semicolon, tab, or pipe delimiters.
- Preview columns, inferred types, and the first rows before writing.
- Import into an existing table with column mapping, or create a new table from
  the file using inferred types.
- Optional upsert key for existing tables.
- Writes directly and does not create a backup automatically.
- Unmapped source columns are ignored and reported.

```bash
dbgo import . people --file people.csv --preview
dbgo import . people --file people.csv
dbgo import . --file people.csv --create-table
dbgo import . people --file people.csv --upsert-key domain
dbgo import . people --file people.csv --map "Full Name=name"
dbgo import . people --file people.csv --no-header --delimiter ';'
```

- Empty CSV cells become NULL.
- JSON can be an object or an array of objects; JSONL is one object per line.
- Default mapping matches source and target column names case-insensitively.
- Upsert requires a PRIMARY KEY or UNIQUE constraint on the key column.
- Column names are sanitized when creating a table from CSV headers.
- Numeric-looking CSV columns with leading zeros are inferred as TEXT so values
  like `00123` are preserved.

## D1 export

Generate a deployment bundle for Cloudflare D1. This is a one-way export; DBGo
does not deploy it for you.

- **GUI:** click **D1 Export** in the header, choose a folder, optionally change
  the D1 database name and overwrite setting, then click **Export**.
- **CLI:**

```bash
dbgo d1 export . --output ./d1 --name my-db
dbgo d1 export . --output ./d1 --name my-db --force
```

The bundle contains:

```text
d1/
├── schema.sql        # tables, indexes, views, triggers
├── data-0001.sql     # batched INSERT statements
├── wrangler.toml     # D1 binding template
├── manifest.json     # tables, row counts, warnings
└── README.md         # Wrangler commands
```

Behavior:

- Internal `_dbgo_*` and `sqlite_*` objects are excluded.
- Tables are ordered by foreign-key dependencies.
- Data is split into files at roughly 500 rows or 500 KB.
- TEXT is SQL-escaped, BLOBs use `X'...'` hex literals, and NULL is preserved.
- Virtual tables and their shadow tables are skipped with a warning.
- Foreign-key cycles are reported and fall back to alphabetical table order.
- `wrangler.toml` contains a placeholder `database_id` for you to fill in.
- No automatic deploy: follow the generated README or run Wrangler yourself.

## Sort, filter, inspect

- Click a column header to cycle ascending → descending → unsorted.
- Use the filter bar for `contains`, comparisons, or null checks.
- Click any row to open the detail panel, copy it as JSON or CSV, or edit/delete
  it as a pending change.
- Pending rows carry an accent edge until you **Save changes** or **Discard**.

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

- Postgres and MySQL drivers
- Installers and release CI

## License

MIT
