//! Compilação de uma consulta para um único texto SQL.
//!
//! # Dialeto adotado
//!
//! Escolhemos um subconjunto SQL simples e portável, próximo do que motores
//! colunares modernos aceitam:
//!
//! - a consulta vira uma cadeia de CTEs `s0`, `s1`, ... (`WITH`), e o resultado
//!   final é `SELECT * FROM` da última;
//! - o CTE inicial é sempre `SELECT * FROM <fonte>`; cada passo lê o anterior;
//! - identificadores (fontes, colunas) vão entre aspas duplas, dobrando aspas
//!   internas; literais de texto vão entre apóstrofos, dobrando apóstrofos;
//! - projeções usam `SELECT * EXCEPT (...)` para remover colunas;
//! - conversão de tipo usa `CAST(... AS ...)`, com `FULL OUTER JOIN` para a
//!   junção completa e `COUNT(DISTINCT ...)` para contagem distinta;
//! - expressões fornecidas pelo usuário (`Filter`, `AddColumn::expr` e
//!   `RawSql`) entram no texto sem reescrita, pois já são SQL; em `RawSql` o
//!   identificador `prev` é trocado pelo nome do CTE anterior.

use std::fmt;

use crate::model::{AggFunc, Aggregation, DataType, Direction, JoinKind, Query, Step};
use crate::sql::{quote_ident, quote_literal};

/// Erro de compilação de uma consulta.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompileError {
    /// O nome da fonte é vazio.
    EmptySource,
    /// `SelectColumns` ou `RemoveColumns` vieram sem colunas.
    EmptyColumns,
    /// `GroupBy` veio sem chaves e sem agregações.
    EmptyGroupBy,
    /// Algum identificador (coluna, nome de agregação ou fonte de junção) é vazio.
    EmptyIdentifier,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptySource => "o nome da fonte não pode ser vazio",
            Self::EmptyColumns => "a lista de colunas não pode ser vazia",
            Self::EmptyGroupBy => "group_by exige ao menos uma chave ou uma agregação",
            Self::EmptyIdentifier => "identificador não pode ser vazio",
        };
        f.write_str(message)
    }
}

impl std::error::Error for CompileError {}

/// Compila a consulta em um único texto SQL, com um CTE por passo.
///
/// Uma consulta sem passos é válida e gera apenas o CTE `s0`.
pub fn compile(query: &Query) -> Result<String, CompileError> {
    if query.source.trim().is_empty() {
        return Err(CompileError::EmptySource);
    }

    let mut bodies = Vec::with_capacity(query.steps.len() + 1);
    bodies.push(format!("SELECT * FROM {}", quote_ident(&query.source)));

    for (index, step) in query.steps.iter().enumerate() {
        let previous = format!("s{index}");
        bodies.push(compile_step(step, &previous)?);
    }

    let mut sql = String::new();
    for (index, body) in bodies.iter().enumerate() {
        if index == 0 {
            sql.push_str("WITH ");
        } else {
            sql.push_str(",\n     ");
        }
        sql.push_str(&format!("s{index} AS ({body})"));
    }
    let last = bodies.len() - 1;
    sql.push_str(&format!("\nSELECT * FROM s{last}"));
    Ok(sql)
}

/// Traduz um passo em um corpo de CTE que lê de `previous`.
fn compile_step(step: &Step, previous: &str) -> Result<String, CompileError> {
    let body = match step {
        Step::SelectColumns(columns) => {
            let columns = column_list(columns)?;
            format!("SELECT {columns} FROM {previous}")
        }
        Step::RemoveColumns(columns) => {
            let columns = column_list(columns)?;
            format!("SELECT * EXCEPT ({columns}) FROM {previous}")
        }
        Step::RenameColumns(renames) => {
            if renames.is_empty() {
                format!("SELECT * FROM {previous}")
            } else {
                let mut removed = Vec::with_capacity(renames.len());
                let mut aliases = Vec::with_capacity(renames.len());
                for (old, new) in renames {
                    let old = ident(old)?;
                    let new = ident(new)?;
                    removed.push(old.clone());
                    aliases.push(format!("{old} AS {new}"));
                }
                format!(
                    "SELECT * EXCEPT ({}), {} FROM {previous}",
                    removed.join(", "),
                    aliases.join(", ")
                )
            }
        }
        Step::ChangeType { column, to } => {
            let column = ident(column)?;
            format!(
                "SELECT * EXCEPT ({column}), CAST({column} AS {}) AS {column} FROM {previous}",
                data_type_sql(*to)
            )
        }
        Step::Filter(expr) => format!("SELECT * FROM {previous} WHERE {expr}"),
        Step::AddColumn { name, expr } => {
            let name = ident(name)?;
            format!("SELECT *, {expr} AS {name} FROM {previous}")
        }
        Step::Sort(keys) => {
            if keys.is_empty() {
                format!("SELECT * FROM {previous}")
            } else {
                let mut order = Vec::with_capacity(keys.len());
                for (column, direction) in keys {
                    let column = ident(column)?;
                    order.push(format!("{column} {}", direction_sql(*direction)));
                }
                format!("SELECT * FROM {previous} ORDER BY {}", order.join(", "))
            }
        }
        Step::GroupBy { keys, aggregations } => {
            if keys.is_empty() && aggregations.is_empty() {
                return Err(CompileError::EmptyGroupBy);
            }
            let mut projection = Vec::with_capacity(keys.len() + aggregations.len());
            for key in keys {
                projection.push(ident(key)?);
            }
            for aggregation in aggregations {
                projection.push(aggregation_sql(aggregation)?);
            }
            let mut body = format!("SELECT {} FROM {previous}", projection.join(", "));
            if !keys.is_empty() {
                let keys = keys
                    .iter()
                    .map(|key| ident(key))
                    .collect::<Result<Vec<_>, _>>()?;
                body.push_str(" GROUP BY ");
                body.push_str(&keys.join(", "));
            }
            body
        }
        Step::Join {
            kind,
            other,
            left_key,
            right_key,
        } => {
            let other = ident(other)?;
            let left_key = ident(left_key)?;
            let right_key = ident(right_key)?;
            format!(
                "SELECT * FROM {previous} {} {other} ON {previous}.{left_key} = {other}.{right_key}",
                join_sql(*kind)
            )
        }
        Step::Append { other } => {
            let other = ident(other)?;
            format!("SELECT * FROM {previous} UNION ALL SELECT * FROM {other}")
        }
        Step::Distinct => format!("SELECT DISTINCT * FROM {previous}"),
        Step::ReplaceValues { column, from, to } => {
            let column = ident(column)?;
            let from = quote_literal(from);
            let to = quote_literal(to);
            format!(
                "SELECT * EXCEPT ({column}), CASE WHEN {column} = {from} THEN {to} ELSE {column} END AS {column} FROM {previous}"
            )
        }
        Step::RawSql(sql) => replace_prev(sql, previous),
    };

    Ok(body)
}

/// Cita uma lista não vazia de identificadores.
fn column_list(columns: &[String]) -> Result<String, CompileError> {
    if columns.is_empty() {
        return Err(CompileError::EmptyColumns);
    }
    let quoted = columns
        .iter()
        .map(|column| ident(column))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quoted.join(", "))
}

/// Valida e cita um identificador.
fn ident(name: &str) -> Result<String, CompileError> {
    if name.trim().is_empty() {
        return Err(CompileError::EmptyIdentifier);
    }
    Ok(quote_ident(name))
}

/// Traduz uma agregação para `função(coluna) AS nome`.
fn aggregation_sql(aggregation: &Aggregation) -> Result<String, CompileError> {
    let name = ident(&aggregation.name)?;
    let argument = match (&aggregation.func, &aggregation.column) {
        (_, Some(column)) => ident(column)?,
        (AggFunc::Count, None) => "*".to_string(),
        _ => return Err(CompileError::EmptyIdentifier),
    };
    let call = match aggregation.func {
        AggFunc::Sum => format!("SUM({argument})"),
        AggFunc::Avg => format!("AVG({argument})"),
        AggFunc::Min => format!("MIN({argument})"),
        AggFunc::Max => format!("MAX({argument})"),
        AggFunc::Count => format!("COUNT({argument})"),
        AggFunc::CountDistinct => format!("COUNT(DISTINCT {argument})"),
    };
    Ok(format!("{call} AS {name}"))
}

/// Tipo SQL correspondente a cada tipo do modelo.
fn data_type_sql(data_type: DataType) -> &'static str {
    match data_type {
        DataType::Int => "BIGINT",
        DataType::Float => "DOUBLE",
        DataType::Decimal => "DECIMAL(38,9)",
        DataType::Bool => "BOOLEAN",
        DataType::Text => "VARCHAR",
        DataType::Date => "DATE",
        DataType::DateTime => "TIMESTAMP",
        DataType::Duration => "INTERVAL",
    }
}

/// Direção SQL de ordenação.
fn direction_sql(direction: Direction) -> &'static str {
    match direction {
        Direction::Asc => "ASC",
        Direction::Desc => "DESC",
    }
}

/// Palavras-chave SQL de cada tipo de junção.
fn join_sql(kind: JoinKind) -> &'static str {
    match kind {
        JoinKind::Inner => "INNER JOIN",
        JoinKind::Left => "LEFT JOIN",
        JoinKind::Right => "RIGHT JOIN",
        JoinKind::Full => "FULL OUTER JOIN",
    }
}

/// Troca o identificador `prev` pelo CTE anterior, sem tocar em texto entre aspas.
fn replace_prev(text: &str, previous: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    let mut quote: Option<char> = None;

    for character in text.chars() {
        if let Some(delimiter) = quote {
            out.push(character);
            if character == delimiter {
                quote = None;
            }
            continue;
        }

        if character == '\'' || character == '"' {
            push_word(&mut out, &mut word, previous);
            quote = Some(character);
            out.push(character);
        } else if character.is_alphanumeric() || character == '_' {
            word.push(character);
        } else {
            push_word(&mut out, &mut word, previous);
            out.push(character);
        }
    }

    push_word(&mut out, &mut word, previous);
    out
}

/// Descarrega a palavra acumulada, trocando-a por `previous` quando for `prev`.
fn push_word(out: &mut String, word: &mut String, previous: &str) {
    if word.is_empty() {
        return;
    }
    if word == "prev" {
        out.push_str(previous);
    } else {
        out.push_str(word);
    }
    word.clear();
}
