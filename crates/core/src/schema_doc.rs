//! Generation of the schema.md snapshot.

use super::*;

/// Render the schema graph as the Markdown format used by schema.md.
pub(crate) fn schema_markdown(graph: &SchemaGraph) -> String {
    let mut output = String::from("# Schema\n\n");
    if graph.tables.is_empty() {
        output.push_str("No tables yet.\n");
        return output;
    }

    for table in &graph.tables {
        output.push_str(&format!("## {}\n\n", md_escape(&table.name)));
        output.push_str("| Column | Type | Constraints |\n|---|---|---|\n");
        for column in &table.columns {
            let mut constraints = Vec::new();
            if column.primary_key {
                constraints.push("PK");
            }
            if column.foreign_key {
                constraints.push("FK");
            }
            if column.not_null {
                constraints.push("NOT NULL");
            }
            output.push_str(&format!(
                "| {} | {} | {} |\n",
                md_escape(&column.name),
                md_escape(&column.type_name),
                constraints.join(", ")
            ));
        }
        output.push('\n');
    }

    if !graph.relationships.is_empty() {
        output.push_str("## Relationships\n\n");
        for relationship in &graph.relationships {
            output.push_str(&format!(
                "- `{}.{}` → `{}.{}` (ON DELETE {})\n",
                md_escape(&relationship.from_table),
                md_escape(&relationship.from_column),
                md_escape(&relationship.to_table),
                md_escape(&relationship.to_column),
                relationship.on_delete
            ));
        }
        output.push('\n');
    }

    output
}

/// Escape Markdown table cell separators.
pub(crate) fn md_escape(value: &str) -> String {
    value.replace('|', "\\|")
}
