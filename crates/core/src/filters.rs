//! Compilation of legacy filters and AND/OR/NOT filter trees into SQL.

use super::*;

/// Build a parameterized WHERE condition for one column, operator, and value.
pub(crate) fn filter_condition_clause(
    column_name: &str,
    op: FilterOp,
    value: Option<&str>,
    columns: &[ColumnInfo],
) -> Result<(String, Vec<rusqlite::types::Value>)> {
    if !columns.iter().any(|column| column.name == column_name) {
        return Err(DbError::UnknownColumn(column_name.to_string()));
    }
    let column = quote_ident(column_name);
    let mut params = Vec::new();

    let clause = match op {
        FilterOp::Contains => {
            let value = value.ok_or(DbError::MissingFilterValue(op))?;
            params.push(rusqlite::types::Value::Text(format!(
                "%{}%",
                escape_like(value)
            )));
            format!("CAST({column} AS TEXT) LIKE ? ESCAPE '\\'")
        }
        FilterOp::Eq
        | FilterOp::Neq
        | FilterOp::Gt
        | FilterOp::Gte
        | FilterOp::Lt
        | FilterOp::Lte => {
            let operator = match op {
                FilterOp::Eq => "=",
                FilterOp::Neq => "<>",
                FilterOp::Gt => ">",
                FilterOp::Gte => ">=",
                FilterOp::Lt => "<",
                FilterOp::Lte => "<=",
                _ => unreachable!(),
            };
            let value = value.ok_or(DbError::MissingFilterValue(op))?;
            params.push(rusqlite::types::Value::Text(value.to_string()));
            format!("{column} {operator} ?")
        }
        FilterOp::IsNull => format!("{column} IS NULL"),
        FilterOp::NotNull => format!("{column} IS NOT NULL"),
    };

    Ok((clause, params))
}

/// Build a full WHERE clause for a legacy single-condition filter.
pub(crate) fn filter_clause(
    filter: &FilterSpec,
    columns: &[ColumnInfo],
) -> Result<(String, Vec<rusqlite::types::Value>)> {
    let (clause, params) =
        filter_condition_clause(&filter.column, filter.op, filter.value.as_deref(), columns)?;
    Ok((format!(" WHERE {clause}"), params))
}

/// Recursively build a parameterized WHERE condition for an AND/OR/NOT filter tree.
pub(crate) fn filter_expr_clause(
    filter: &FilterExpr,
    columns: &[ColumnInfo],
) -> Result<(String, Vec<rusqlite::types::Value>)> {
    match filter {
        FilterExpr::Condition { column, op, value } => {
            let (clause, params) = filter_condition_clause(column, *op, value.as_deref(), columns)?;
            Ok((format!("({clause})"), params))
        }
        FilterExpr::And { filters } | FilterExpr::Or { filters } => {
            if filters.is_empty() {
                return Err(DbError::InvalidDesign(
                    "AND/OR filters need at least one condition".to_string(),
                ));
            }
            let joiner = if matches!(filter, FilterExpr::And { .. }) {
                " AND "
            } else {
                " OR "
            };
            let mut clauses = Vec::with_capacity(filters.len());
            let mut params = Vec::new();
            for child in filters {
                let (clause, child_params) = filter_expr_clause(child, columns)?;
                clauses.push(clause);
                params.extend(child_params);
            }
            Ok((format!("({})", clauses.join(joiner)), params))
        }
        FilterExpr::Not { filter } => {
            let (clause, params) = filter_expr_clause(filter, columns)?;
            Ok((format!("NOT {clause}"), params))
        }
    }
}
