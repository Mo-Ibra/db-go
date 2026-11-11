# Frontend architecture

DBGo is a single-window React application organized around screens, feature
hooks, and a shared UI component layer.

## Composition

```text
App
├── WelcomeScreen                     # no database open
└── DatabaseWorkspace                 # keyed by db.path
    ├── AppHeader
    ├── NavigationRail
    ├── TableSidebar                  # Data and Structure only
    ├── DatabaseScreen
    │   ├── StatusBanners
    │   ├── DataScreen
    │   ├── StructureScreen
    │   ├── DiagramScreen
    │   ├── SqlScreen
    │   └── StatusFooter
    └── AppDialogs
```

`DatabaseWorkspace` is keyed by `db.path`. Opening another database unmounts the
old workspace and mounts a fresh one, so table state, pending edits, query state,
and dialogs do not need a large manual reset routine.

## Screens

- **Welcome** — create/open database and recent databases.
- **Data** — table sidebar, toolbar, filter bar, grid, row detail, pending changes.
- **Structure** — table sidebar, columns, indexes, foreign keys, DDL.
- **Diagram** — full-width schema diagram.
- **Cards** — schema-driven card explorer with five layouts, pagination, search, sort, and a read-only detail drawer.
- **SQL** — editor, query history, query plan, results, export.

Import, backups, migrations, D1 export, table design, and row editing remain
modal dialogs because they are tasks rather than destinations.

## Folders

```text
src/
├── app/
│   ├── App.tsx
│   ├── navigation/
│   └── providers/
├── screens/
│   ├── welcome/
│   └── database/
│       ├── data/
│       ├── structure/
│       ├── diagram/
│       ├── cards/
│       ├── sql/
│       └── results/
├── components/
│   ├── ui/                  # Button, IconButton, Badge, Select, EmptyState…
│   ├── layout/              # AppHeader, TableSidebar, StatusFooter
│   └── feedback/            # StatusBanners
├── features/
│   ├── connection/
│   ├── tables/
│   ├── schema/
│   ├── query/
│   ├── diagram/
│   ├── cards/
│   ├── project/
│   └── workspace/
└── lib/
```

Feature hooks and feature-specific components live with their feature. The UI
folder contains only reusable primitives.

## State ownership

| Concern | Owner |
|---|---|
| Open database, recents, session errors/notices | `useDatabaseSession` |
| Recent database persistence | `useRecentDatabases` |
| Table selection, pagination, sorting, filtering | `useTableData` |
| Pending inserts/updates/deletes, row editor, save/discard | `usePendingMutations` |
| Row selection, ranges, clipboard/delete selection | `useRowSelection` |
| Composition of pending mutations and row selection | `usePendingEdits` |
| SQL text, result, plan, history | `useQueryEditor` |
| Structure details and stats | `useTableDetails` |
| Migrations, backups, import, D1 dialogs | `useProjectDialogs` |
| Card templates and localStorage persistence | `useCardTemplates` |
| Card page loading, search, sort | `useCardPage` |
| Schema/index/navigation actions | `useSchemaActions` |
| External database change polling | `useExternalDataSync` |
| Ctrl/Cmd+O shortcut | `useOpenDatabaseShortcut` |
| Cross-feature coordination | `useWorkspaceController` |

## Styling conventions

- Reusable behavior belongs in `components/ui` primitives: `Button`, `IconButton`,
  `Badge`, `Select`, `EmptyState`.
- Repeated Tailwind form patterns live in `src/index.css` as component classes:
  `.btn`, `.btn-neutral`, `.btn-primary`, `.btn-ghost`, `.field-input`,
  `.field-select`, `.field-label`, and `.card`.
- Do not define `const BTN = ...` or similar Tailwind class strings in feature
  components.
- Icons come from `lucide-react`. Icon-only buttons must have `aria-label` and
  `title`.

## Data flow

- `useWorkspaceController` composes feature hooks and exposes one controller
  through `WorkspaceContext`.
- Screens and components read only the fields they need from `useWorkspace()`.
- Async requests keep request IDs where a stale response could overwrite newer
  state.
- Pending edits stay client-side until `api.saveChanges` succeeds. External
  polling never reloads a table while pending edits exist; it shows a notice.

## Adding a feature

1. Put data fetching and mutations in a feature hook.
2. Add the hook result to `useWorkspaceController` only if a screen or dialog needs it.
3. Keep presentational components free of direct `api` calls.
4. Add a screen under `screens/` or a dialog under the owning feature.
