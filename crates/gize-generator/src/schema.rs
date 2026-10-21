//! Offline replay of forward SQL migrations. Unsupported DDL is an error, never an
//! assumption that the manifest matches. We track columns/types/nullability, not data.
use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use gize_core::Dialect;
use sqlparser::ast::{
    AlterColumnOperation, AlterTableOperation, ColumnDef, ColumnOption, DataType, Ident,
    ObjectName, ObjectType, Statement, TableConstraint,
};
use sqlparser::dialect::{MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::Parser;

#[derive(Debug, Clone)]
pub(crate) struct Column {
    pub ty: String,
    pub nullable: bool,
}
pub(crate) type Table = BTreeMap<String, Column>;

fn ident(name: &Ident, dialect: Dialect) -> String {
    if dialect == Dialect::Sqlite || (dialect == Dialect::Postgres && name.quote_style.is_none()) {
        name.value.to_lowercase()
    } else {
        name.value.clone()
    }
}

fn table_name(name: &ObjectName, dialect: Dialect) -> Result<String> {
    ensure!(
        name.0.len() == 1,
        "schema-qualified table names are not supported by offline diff: {name}"
    );
    Ok(ident(&name.0[0], dialect))
}

/// Normalize only well-known SQL aliases; preserve size, precision and timezone semantics.
pub(crate) fn normalize_type(ty: &str, dialect: Dialect) -> String {
    let ty = ty.to_ascii_uppercase().replace(' ', "");
    match (dialect, ty.as_str()) {
        (_, "INT" | "INT4" | "INTEGER") => "INTEGER".into(),
        (_, "BOOL" | "BOOLEAN") => "BOOLEAN".into(),
        (Dialect::Postgres, "INT8" | "BIGINT") => "BIGINT".into(),
        (Dialect::Postgres, "FLOAT8" | "DOUBLEPRECISION") => "DOUBLEPRECISION".into(),
        (Dialect::Postgres, "TIMESTAMPWITHTIMEZONE" | "TIMESTAMPTZ") => "TIMESTAMPTZ".into(),
        (Dialect::MySql, "DOUBLEPRECISION" | "DOUBLE") => "DOUBLE".into(),
        (Dialect::MySql, "TINYINT(1)") => "BOOLEAN".into(),
        _ => ty,
    }
}

fn column(ty: &DataType, options: impl Iterator<Item = ColumnOption>, dialect: Dialect) -> Column {
    Column {
        ty: normalize_type(&ty.to_string(), dialect),
        nullable: !options.into_iter().any(|o| {
            matches!(
                o,
                ColumnOption::NotNull
                    | ColumnOption::Unique {
                        is_primary: true,
                        ..
                    }
            )
        }),
    }
}

fn add_column(
    table: &mut Table,
    def: &ColumnDef,
    if_not_exists: bool,
    dialect: Dialect,
) -> Result<()> {
    let name = ident(&def.name, dialect);
    match table.entry(name) {
        std::collections::btree_map::Entry::Occupied(entry) => {
            ensure!(
                if_not_exists,
                "column {} already exists in migration history",
                entry.key()
            );
        }
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(column(
                &def.data_type,
                def.options.iter().map(|o| o.option.clone()),
                dialect,
            ));
        }
    }
    Ok(())
}

/// SQLx orders versions numerically and does not apply .down.sql files in forward replay.
pub(crate) fn replay(dir: &Path, dialect: Dialect) -> Result<BTreeMap<String, Table>> {
    if !dir.exists() {
        return Ok(BTreeMap::new());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().is_none_or(|ext| ext != "sql") || !path.is_file() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy();
        if name.ends_with(".down.sql") {
            continue;
        }
        let version = name
            .split_once('_')
            .and_then(|(v, _)| v.parse::<i64>().ok())
            .with_context(|| {
                format!("invalid migration filename {name}: expected <version>_<name>.sql")
            })?;
        files.push((version, path));
    }
    files.sort_by_key(|(version, _)| *version);
    for pair in files.windows(2) {
        ensure!(
            pair[0].0 != pair[1].0,
            "duplicate migration version {}: {} and {}",
            pair[0].0,
            pair[0].1.display(),
            pair[1].1.display()
        );
    }
    let parser_dialect: &dyn sqlparser::dialect::Dialect = match dialect {
        Dialect::Postgres => &PostgreSqlDialect {},
        Dialect::Sqlite => &SQLiteDialect {},
        Dialect::MySql => &MySqlDialect {},
    };
    let mut tables = BTreeMap::new();
    for (_, path) in files {
        let sql = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        ensure!(
            dialect != Dialect::MySql || (!sql.contains("/*!") && !sql.contains("/*M!")),
            "cannot replay migration {}: MySQL executable comment markers are not supported by offline diff",
            path.display()
        );
        let statements = Parser::parse_sql(parser_dialect, &sql).with_context(|| {
            format!(
                "cannot parse migration {}; offline schema diff stopped",
                path.display()
            )
        })?;
        for (index, statement) in statements.into_iter().enumerate() {
            apply(&mut tables, statement, dialect).with_context(|| format!(
                "cannot replay migration {}, statement {}; offline schema diff stopped (review this SQL manually)", path.display(), index + 1
            ))?;
        }
    }
    Ok(tables)
}

fn apply(
    tables: &mut BTreeMap<String, Table>,
    statement: Statement,
    dialect: Dialect,
) -> Result<()> {
    match statement {
        Statement::CreateTable(create) => {
            let name = table_name(&create.name, dialect)?;
            ensure!(
                !create.or_replace
                    && !create.temporary
                    && create.query.is_none()
                    && create.like.is_none()
                    && create.clone.is_none()
                    && !create.columns.is_empty(),
                "unsupported CREATE TABLE form for {name}; explicit permanent columns are required"
            );
            if tables.contains_key(&name) {
                ensure!(
                    create.if_not_exists,
                    "table {name} already exists in migration history"
                );
                return Ok(());
            }
            let mut table = Table::new();
            for def in &create.columns {
                add_column(&mut table, def, false, dialect)?;
            }
            for constraint in create.constraints {
                if let TableConstraint::PrimaryKey { columns, .. } = constraint {
                    for key in columns {
                        table
                            .get_mut(&ident(&key, dialect))
                            .context("unknown primary-key column")?
                            .nullable = false;
                    }
                }
            }
            tables.insert(name, table);
        }
        Statement::AlterTable {
            name,
            if_exists,
            operations,
            only,
            location,
            on_cluster,
        } => {
            ensure!(
                !only && location.is_none() && on_cluster.is_none(),
                "unsupported ALTER TABLE modifier"
            );
            let mut name = table_name(&name, dialect)?;
            if !tables.contains_key(&name) && if_exists {
                return Ok(());
            }
            for op in operations {
                if let AlterTableOperation::RenameTable { table_name: next } = &op {
                    let next = table_name(next, dialect)?;
                    ensure!(!tables.contains_key(&next), "table {next} already exists");
                    let table = tables
                        .remove(&name)
                        .with_context(|| format!("unknown table {name}"))?;
                    tables.insert(next.clone(), table);
                    name = next;
                    continue;
                }
                let table = tables
                    .get_mut(&name)
                    .with_context(|| format!("unknown table {name}"))?;
                match op {
                    AlterTableOperation::AddConstraint(constraint) => {
                        if let TableConstraint::PrimaryKey { columns, .. } = constraint {
                            for key in columns {
                                table
                                    .get_mut(&ident(&key, dialect))
                                    .context("unknown primary-key column")?
                                    .nullable = false;
                            }
                        }
                        // FK, CHECK and UNIQUE do not change the tracked column shape.
                    }
                    // PostgreSQL/MySQL retain NOT NULL after dropping a primary key.
                    AlterTableOperation::DropConstraint { .. }
                    | AlterTableOperation::RenameConstraint { .. }
                    | AlterTableOperation::DropPrimaryKey => {}
                    AlterTableOperation::AddColumn {
                        column_def,
                        if_not_exists,
                        ..
                    } => add_column(table, &column_def, if_not_exists, dialect)?,
                    AlterTableOperation::DropColumn {
                        column_name,
                        if_exists,
                        cascade,
                    } => {
                        ensure!(
                            !cascade,
                            "DROP COLUMN CASCADE is not supported by offline diff"
                        );
                        let key = ident(&column_name, dialect);
                        ensure!(
                            table.remove(&key).is_some() || if_exists,
                            "unknown column {key}"
                        );
                    }
                    AlterTableOperation::RenameColumn {
                        old_column_name,
                        new_column_name,
                    } => {
                        let old = ident(&old_column_name, dialect);
                        let new = ident(&new_column_name, dialect);
                        ensure!(!table.contains_key(&new), "column {new} already exists");
                        let col = table
                            .remove(&old)
                            .with_context(|| format!("unknown column {old}"))?;
                        table.insert(new, col);
                    }
                    AlterTableOperation::AlterColumn { column_name, op } => {
                        let key = ident(&column_name, dialect);
                        let col = table
                            .get_mut(&key)
                            .with_context(|| format!("unknown column {key}"))?;
                        match op {
                            AlterColumnOperation::SetDataType { data_type, .. } => {
                                col.ty = normalize_type(&data_type.to_string(), dialect)
                            }
                            AlterColumnOperation::SetNotNull => col.nullable = false,
                            AlterColumnOperation::DropNotNull => col.nullable = true,
                            AlterColumnOperation::SetDefault { .. }
                            | AlterColumnOperation::DropDefault => {}
                            other => bail!("unsupported ALTER COLUMN: {other}"),
                        }
                    }
                    AlterTableOperation::ModifyColumn {
                        col_name,
                        data_type,
                        options,
                        ..
                    } => {
                        let key = ident(&col_name, dialect);
                        ensure!(table.contains_key(&key), "unknown column {key}");
                        table.insert(key, column(&data_type, options.into_iter(), dialect));
                    }
                    AlterTableOperation::ChangeColumn {
                        old_name,
                        new_name,
                        data_type,
                        options,
                        ..
                    } => {
                        let old = ident(&old_name, dialect);
                        let new = ident(&new_name, dialect);
                        ensure!(
                            old == new || !table.contains_key(&new),
                            "column {new} already exists"
                        );
                        ensure!(table.remove(&old).is_some(), "unknown column {old}");
                        table.insert(new, column(&data_type, options.into_iter(), dialect));
                    }
                    other => bail!("unsupported ALTER TABLE operation: {other}"),
                }
            }
        }
        Statement::Drop {
            object_type: ObjectType::Table,
            names,
            if_exists,
            cascade,
            ..
        } => {
            ensure!(
                !cascade,
                "DROP TABLE CASCADE is not supported by offline diff"
            );
            for name in names {
                let key = table_name(&name, dialect)?;
                ensure!(
                    tables.remove(&key).is_some() || if_exists,
                    "unknown table {key}"
                );
            }
        }
        Statement::Pragma { name, .. } if dialect == Dialect::Sqlite => {
            let key = table_name(&name, dialect)?;
            ensure!(
                matches!(key.as_str(), "foreign_keys" | "defer_foreign_keys"),
                "unsupported PRAGMA for offline diff: {name}"
            );
        }
        // These statements do not change the column shape. Data is never executed here.
        Statement::DropFunction { option, .. } | Statement::DropTrigger { option, .. } => {
            ensure!(
                option != Some(sqlparser::ast::ReferentialAction::Cascade),
                "DROP FUNCTION/TRIGGER CASCADE is not supported by offline diff"
            );
        }
        // Without CASCADE the database refuses to drop a type a column still uses.
        Statement::Drop {
            object_type: ObjectType::Type,
            cascade,
            ..
        } => {
            ensure!(
                !cascade,
                "DROP TYPE CASCADE is not supported by offline diff"
            );
        }
        Statement::CreateType { .. }
        | Statement::CreateExtension { .. }
        | Statement::CreateView { .. }
        | Statement::CreateFunction(_)
        | Statement::CreateTrigger { .. }
        | Statement::Comment { .. }
        | Statement::Grant { .. }
        | Statement::Revoke { .. }
        | Statement::Insert(_)
        | Statement::Update { .. }
        | Statement::Delete(_)
        | Statement::CreateIndex(_)
        | Statement::Drop {
            object_type: ObjectType::Index | ObjectType::View,
            ..
        }
        | Statement::StartTransaction { .. }
        | Statement::Commit { .. } => {}
        other => bail!("unsupported SQL statement for offline schema diff: {other}"),
    }
    Ok(())
}
