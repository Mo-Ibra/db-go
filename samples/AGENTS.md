# Working with samples

The database is `companies.db` in this folder. Never edit the database file directly.

## Discover the schema

- `dbgo graph . --json` — full tables, columns, and relationships
- `dbgo schema . companies --json` — columns for one table
- `dbgo tables .` — table list
- `schema.md` is a generated snapshot; the CLI is authoritative.

## Read

- `dbgo query . "select * from companies limit 20" --json`
- `dbgo page . companies --limit 50`
- `dbgo count . companies`
- `dbgo explain . "select ..." --json` — query plan without executing it
- `dbgo stats . --json` — page, table, and index sizes

## Write data

Use JSONL files, one JSON object per line:

```jsonl
{"name":"Acme","domain":"acme.test","city":"Berlin"}
{"name":"Globex","domain":"globex.test","city":"Austin"}
```

```bash
dbgo insert . companies --file rows.jsonl
dbgo upsert . companies --key domain --file rows.jsonl
```

Updates and deletes use a JSON filter and require `--yes`:

```bash
dbgo update . companies \
  --where '{"column":"domain","op":"eq","value":"acme.test"}' \
  --set '{"city":"Munich"}' \
  --yes

dbgo delete . companies \
  --where '{"column":"domain","op":"eq","value":"globex.test"}' \
  --yes
```

- `upsert` needs a UNIQUE or PRIMARY KEY constraint on the key column.
- Updating primary key columns is not supported.
- Every write is audited. Backups are manual (`dbgo backup .`).

`--where` also accepts an AND/OR/NOT expression tree:

```json
{
  "type": "and",
  "filters": [
    { "type": "condition", "column": "id", "op": "gte", "value": "2" },
    { "type": "or", "filters": [
      { "type": "condition", "column": "city", "op": "eq", "value": "Berlin" },
      { "type": "condition", "column": "city", "op": "eq", "value": "Paris" }
    ] }
  ]
}
```

Node types: `condition`, `and`, `or`, `not`. Old single-condition JSON still
works. Values may be strings, numbers, or booleans.

## Execute arbitrary SQL

`dbgo exec` is read-only unless `--write` and `--yes` are provided.

```bash
dbgo exec . "select * from companies limit 20"
dbgo exec . "update employees set salary = salary + 1" --write --yes
dbgo exec . --file script.sql --write --yes
dbgo exec . "update companies set city = 'Berlin' returning id, city" \
  --write --yes --returning
dbgo exec . "vacuum" --write --yes --no-transaction
```

- Normal writes run in one transaction.
- `--no-transaction` is required for VACUUM, ATTACH, write PRAGMAs, or scripts
  with their own BEGIN/COMMIT.
- `--returning` executes a single statement and returns rows.
- Writes get one audit session; no automatic backup.
- `--limit` caps returned rows.

## DDL and maintenance

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

- Drop table/trigger require `--yes`.
- Schema changes write migrations and refresh `schema.md`.
- `vacuum`, `analyze`, and write PRAGMAs are audited but do not create migrations.
- Use `dbgo exec` for column type changes, table rebuilds, ATTACH, and other
  arbitrary DDL.

## Export

```bash
dbgo export . companies --format csv --output companies.csv
dbgo export . companies --format json --where '{"column":"id","op":"gte","value":1}'
dbgo export . --query "select * from companies order by id" --format json --output result.json
```

- Exports are read-only and do not create backups.
- Omit `--output` to write data to stdout.
- BLOB values are base64-encoded.

## Import

```bash
dbgo import . people --file people.csv --preview
dbgo import . people --file people.csv
dbgo import . --file people.csv --create-table
dbgo import . people --file people.csv --upsert-key domain
dbgo import . people --file people.csv --map "Full Name=name"
```

- CSV, JSON, and JSONL are supported.
- Empty CSV cells become NULL.
- Default mapping matches source and target column names case-insensitively.
- `--map` overrides matching columns without disabling default mapping; use
  `--map "Source="` to skip a source column.
- Create-table imports infer column types and sanitize column names.
- Upsert requires a PRIMARY KEY or UNIQUE constraint on the key column.
- Imports write directly; backups are manual (`dbgo backup .`).

## D1 export

```bash
dbgo d1 export . --output ./d1 --name my-db
```

- Generates `schema.sql`, `data-*.sql`, `wrangler.toml`, `manifest.json`, and `README.md`.
- Internal `_dbgo_*` tables are excluded.
- Tables are ordered by foreign keys; data is split into batches.
- Deploy manually with Wrangler; DBGo does not deploy.
- Use `--force` to overwrite a non-empty output directory.

## GUI edits

The desktop app stages row edits and writes them only when the user clicks
**Save changes**. Each save creates one audit session. Backups are manual.
The app does not reload the current page while edits are staged, so an agent
can write safely and the user decides when to reload.

Cells can be edited inline, selected in bulk, and copied/pasted as TSV. All of
that stays pending until **Save changes**.

## Create tables

Write a JSON design file, for example `table.json`:

```json
{
  "name": "companies",
  "columns": [
    {
      "name": "id",
      "type_name": "INTEGER",
      "primary_key": true,
      "not_null": true,
      "unique": false,
      "default_value": null,
      "references": null
    },
    {
      "name": "name",
      "type_name": "TEXT",
      "primary_key": false,
      "not_null": true,
      "unique": false,
      "default_value": null,
      "references": null
    }
  ],
  "index_foreign_keys": true
}
```

Then run:

```bash
dbgo create-table . --file table.json
```

## Alter tables

An operations file keeps multiple changes atomic:

```json
{
  "operations": [
    {
      "op": "add_column",
      "column": {
        "name": "country",
        "type_name": "TEXT",
        "primary_key": false,
        "not_null": true,
        "unique": false,
        "default_value": "US",
        "references": null
      }
    },
    { "op": "rename_column", "from": "city", "to": "location" },
    { "op": "drop_column", "name": "legacy_id" }
  ]
}
```

```bash
dbgo alter-table . companies --file alter.json
dbgo alter-table . companies --add-column '{"name":"note","type_name":"TEXT","primary_key":false,"not_null":false,"unique":false,"default_value":null,"references":null}'
dbgo alter-table . companies --rename-column city=location
dbgo alter-table . companies --drop-column legacy_id --yes
dbgo alter-table . companies --file alter.json --sql-only
```

- `--sql-only` validates and prints SQL without writing.
- Dropping a column requires `--yes`.
- Adding a NOT NULL column requires a default.
- Adding a primary-key or UNIQUE column is rejected; create a unique index instead.
- A dropped column cannot be used by an index; drop the index first.

## Indexes and views

```bash
dbgo index create . companies --name idx_companies_city --columns city
dbgo index create . companies --name idx_companies_domain --columns domain --unique
dbgo index list . companies
dbgo index drop . companies idx_companies_city --yes

dbgo view create . active_companies --sql "select * from companies where active = 1"
dbgo view list .
dbgo view drop . active_companies --yes
```

## Migrations

Schema commands write numbered `.sql` files into `migrations/` and record them in
`_dbgo_migrations`. Apply any files added by hand or by another tool:

```bash
dbgo migrations .
dbgo migrate .
```

Migrations run in filename order. Each applied file gets an audit session. Do
not edit a migration that is already recorded as applied.

## Backups and audit

- Backups are manual. Use `dbgo backup .` when you want a snapshot.
- `dbgo backups .` lists snapshots, newest first.
- `dbgo restore . --latest --yes` restores the newest snapshot.
- `dbgo restore . --backup <path> --yes` restores one specific snapshot.
- A restore validates the snapshot. Take a backup first if you need to undo it.
- `dbgo sessions .` lists recent write sessions and their action counts.

## Rules

- Use `dbgo`; never write the SQLite file directly.
- Run `dbgo graph . --json` before changing the schema.
- Use `--json` or `--jsonl` for machine-readable output.
- Run commands from this folder and use `.` as the database path.
- Use `--yes` for update/delete, drop-table, trigger drop, `exec --write`, and
  write PRAGMAs.
- Use `--yes` when dropping columns, indexes, or views.
- Use `--yes` when restoring a backup.
- Prefer `upsert` with a natural key over blind inserts.
- Do not update primary key columns.
- Do not edit a migration that is already applied; add a new numbered file instead.
- Check `dbgo migrations .` before and after changing the schema manually.
