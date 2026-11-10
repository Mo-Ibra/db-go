# Schema

## companies

| Column | Type | Constraints |
|---|---|---|
| id | INTEGER | PK |
| name | TEXT | NOT NULL |
| domain | TEXT |  |
| city | TEXT |  |
| employees | INTEGER |  |
| revenue | REAL |  |
| tags | TEXT |  |
| active | INTEGER | NOT NULL |
| founded | TEXT |  |
| notes | TEXT |  |
| logo | BLOB |  |
| updated_at | TEXT | NOT NULL |

## links

| Column | Type | Constraints |
|---|---|---|
| id | INTEGER | PK |
| company_id | INTEGER | FK, NOT NULL |
| url | TEXT | NOT NULL |
| kind | TEXT | NOT NULL |
| checked_at | TEXT |  |

## Relationships

- `links.company_id` → `companies.id` (ON DELETE NO ACTION)

## Indexes

- `companies`: `idx_companies_city` on `city`
- `companies`: `sqlite_autoindex_companies_1` (unique) on `domain`
- `links`: `idx_links_company` on `company_id`

