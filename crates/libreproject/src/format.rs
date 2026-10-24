//! Representação em arquivo de um projeto.
//!
//! Estes tipos existem apenas para (de)serializar YAML: a API pública usa
//! [`librequery::Query`] e [`librequery::Step`], que continuam sem depender de
//! serde. As conversões entre os dois mundos ficam neste módulo.
//!
//! O formato é canônico por construção: os campos aparecem na ordem declarada
//! nas estruturas, os passos usam a chave `op` em *snake_case* e os nomes dos
//! arquivos vêm dos identificadores.

use librequery::{AggFunc, Aggregation, DataType, Direction, JoinKind, Query, Step};
use serde::{Deserialize, Serialize};

use crate::{SourceDef, SourceKind};

/// Nome do arquivo que descreve o projeto.
pub(crate) const PROJECT_FILE: &str = "project.yaml";

/// Pasta com as definições das fontes de dados.
pub(crate) const SOURCES_DIR: &str = "sources";

/// Pasta com as definições das consultas.
pub(crate) const QUERIES_DIR: &str = "queries";

/// Conteúdo de `project.yaml`.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ProjectFile {
    pub name: String,
    pub format_version: u32,
}

/// Conteúdo de `sources/<id>.yaml`.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SourceFile {
    pub id: String,
    pub kind: SourceKindFile,
    pub path: String,
}

/// Tipo de fonte aceito no arquivo.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SourceKindFile {
    Csv,
    Parquet,
}

/// Conteúdo de `queries/<id>.yaml`.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct QueryFile {
    pub id: String,
    pub source: String,
    #[serde(default)]
    pub steps: Vec<StepFile>,
}

/// Um passo no arquivo, com a operação na chave `op`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub(crate) enum StepFile {
    SelectColumns {
        columns: Vec<String>,
    },
    RemoveColumns {
        columns: Vec<String>,
    },
    RenameColumns {
        renames: Vec<RenameFile>,
    },
    ChangeType {
        column: String,
        to: DataTypeFile,
    },
    Filter {
        expr: String,
    },
    AddColumn {
        name: String,
        expr: String,
    },
    Sort {
        columns: Vec<SortKeyFile>,
    },
    GroupBy {
        keys: Vec<String>,
        #[serde(default)]
        aggregations: Vec<AggregationFile>,
    },
    Join {
        kind: JoinKindFile,
        other: String,
        left_key: String,
        right_key: String,
    },
    Append {
        other: String,
    },
    Distinct,
    ReplaceValues {
        column: String,
        from: String,
        to: String,
    },
    RawSql {
        sql: String,
    },
}

/// Par `from`/`to` de um passo `rename_columns`.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct RenameFile {
    pub from: String,
    pub to: String,
}

/// Coluna e direção de um passo `sort`.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SortKeyFile {
    pub column: String,
    pub direction: DirectionFile,
}

/// Direção de ordenação no arquivo.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DirectionFile {
    Asc,
    Desc,
}

/// Agregação de um passo `group_by`.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AggregationFile {
    pub name: String,
    pub func: AggFuncFile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<String>,
}

/// Função de agregação no arquivo.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AggFuncFile {
    Sum,
    Avg,
    Min,
    Max,
    Count,
    CountDistinct,
}

/// Tipo de dado no arquivo, sempre em minúsculas.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DataTypeFile {
    Int,
    Float,
    Decimal,
    Bool,
    Text,
    Date,
    DateTime,
    Duration,
}

/// Tipo de junção no arquivo.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum JoinKindFile {
    Inner,
    Left,
    Right,
    Full,
}

impl From<SourceKindFile> for SourceKind {
    fn from(kind: SourceKindFile) -> Self {
        match kind {
            SourceKindFile::Csv => SourceKind::Csv,
            SourceKindFile::Parquet => SourceKind::Parquet,
        }
    }
}

impl From<SourceKind> for SourceKindFile {
    fn from(kind: SourceKind) -> Self {
        match kind {
            SourceKind::Csv => SourceKindFile::Csv,
            SourceKind::Parquet => SourceKindFile::Parquet,
        }
    }
}

impl From<SourceFile> for SourceDef {
    fn from(file: SourceFile) -> Self {
        SourceDef {
            id: file.id,
            kind: file.kind.into(),
            path: file.path,
        }
    }
}

impl From<&SourceDef> for SourceFile {
    fn from(source: &SourceDef) -> Self {
        SourceFile {
            id: source.id.clone(),
            kind: source.kind.into(),
            path: source.path.clone(),
        }
    }
}

impl From<QueryFile> for Query {
    fn from(file: QueryFile) -> Self {
        Query {
            id: file.id,
            source: file.source,
            steps: file.steps.into_iter().map(Step::from).collect(),
        }
    }
}

impl From<&Query> for QueryFile {
    fn from(query: &Query) -> Self {
        QueryFile {
            id: query.id.clone(),
            source: query.source.clone(),
            steps: query.steps.iter().map(StepFile::from).collect(),
        }
    }
}

impl From<StepFile> for Step {
    fn from(file: StepFile) -> Self {
        match file {
            StepFile::SelectColumns { columns } => Step::SelectColumns(columns),
            StepFile::RemoveColumns { columns } => Step::RemoveColumns(columns),
            StepFile::RenameColumns { renames } => Step::RenameColumns(
                renames
                    .into_iter()
                    .map(|rename| (rename.from, rename.to))
                    .collect(),
            ),
            StepFile::ChangeType { column, to } => Step::ChangeType {
                column,
                to: to.into(),
            },
            StepFile::Filter { expr } => Step::Filter(expr),
            StepFile::AddColumn { name, expr } => Step::AddColumn { name, expr },
            StepFile::Sort { columns } => Step::Sort(
                columns
                    .into_iter()
                    .map(|key| (key.column, key.direction.into()))
                    .collect(),
            ),
            StepFile::GroupBy { keys, aggregations } => Step::GroupBy {
                keys,
                aggregations: aggregations.into_iter().map(Aggregation::from).collect(),
            },
            StepFile::Join {
                kind,
                other,
                left_key,
                right_key,
            } => Step::Join {
                kind: kind.into(),
                other,
                left_key,
                right_key,
            },
            StepFile::Append { other } => Step::Append { other },
            StepFile::Distinct => Step::Distinct,
            StepFile::ReplaceValues { column, from, to } => {
                Step::ReplaceValues { column, from, to }
            }
            StepFile::RawSql { sql } => Step::RawSql(sql),
        }
    }
}

impl From<&Step> for StepFile {
    fn from(step: &Step) -> Self {
        match step {
            Step::SelectColumns(columns) => StepFile::SelectColumns {
                columns: columns.clone(),
            },
            Step::RemoveColumns(columns) => StepFile::RemoveColumns {
                columns: columns.clone(),
            },
            Step::RenameColumns(renames) => StepFile::RenameColumns {
                renames: renames
                    .iter()
                    .map(|(from, to)| RenameFile {
                        from: from.clone(),
                        to: to.clone(),
                    })
                    .collect(),
            },
            Step::ChangeType { column, to } => StepFile::ChangeType {
                column: column.clone(),
                to: (*to).into(),
            },
            Step::Filter(expr) => StepFile::Filter { expr: expr.clone() },
            Step::AddColumn { name, expr } => StepFile::AddColumn {
                name: name.clone(),
                expr: expr.clone(),
            },
            Step::Sort(keys) => StepFile::Sort {
                columns: keys
                    .iter()
                    .map(|(column, direction)| SortKeyFile {
                        column: column.clone(),
                        direction: (*direction).into(),
                    })
                    .collect(),
            },
            Step::GroupBy { keys, aggregations } => StepFile::GroupBy {
                keys: keys.clone(),
                aggregations: aggregations.iter().map(AggregationFile::from).collect(),
            },
            Step::Join {
                kind,
                other,
                left_key,
                right_key,
            } => StepFile::Join {
                kind: (*kind).into(),
                other: other.clone(),
                left_key: left_key.clone(),
                right_key: right_key.clone(),
            },
            Step::Append { other } => StepFile::Append {
                other: other.clone(),
            },
            Step::Distinct => StepFile::Distinct,
            Step::ReplaceValues { column, from, to } => StepFile::ReplaceValues {
                column: column.clone(),
                from: from.clone(),
                to: to.clone(),
            },
            Step::RawSql(sql) => StepFile::RawSql { sql: sql.clone() },
        }
    }
}

impl From<AggregationFile> for Aggregation {
    fn from(file: AggregationFile) -> Self {
        Aggregation {
            name: file.name,
            func: file.func.into(),
            column: file.column,
        }
    }
}

impl From<&Aggregation> for AggregationFile {
    fn from(aggregation: &Aggregation) -> Self {
        AggregationFile {
            name: aggregation.name.clone(),
            func: aggregation.func.into(),
            column: aggregation.column.clone(),
        }
    }
}

impl From<AggFuncFile> for AggFunc {
    fn from(func: AggFuncFile) -> Self {
        match func {
            AggFuncFile::Sum => AggFunc::Sum,
            AggFuncFile::Avg => AggFunc::Avg,
            AggFuncFile::Min => AggFunc::Min,
            AggFuncFile::Max => AggFunc::Max,
            AggFuncFile::Count => AggFunc::Count,
            AggFuncFile::CountDistinct => AggFunc::CountDistinct,
        }
    }
}

impl From<AggFunc> for AggFuncFile {
    fn from(func: AggFunc) -> Self {
        match func {
            AggFunc::Sum => AggFuncFile::Sum,
            AggFunc::Avg => AggFuncFile::Avg,
            AggFunc::Min => AggFuncFile::Min,
            AggFunc::Max => AggFuncFile::Max,
            AggFunc::Count => AggFuncFile::Count,
            AggFunc::CountDistinct => AggFuncFile::CountDistinct,
        }
    }
}

impl From<DataTypeFile> for DataType {
    fn from(data_type: DataTypeFile) -> Self {
        match data_type {
            DataTypeFile::Int => DataType::Int,
            DataTypeFile::Float => DataType::Float,
            DataTypeFile::Decimal => DataType::Decimal,
            DataTypeFile::Bool => DataType::Bool,
            DataTypeFile::Text => DataType::Text,
            DataTypeFile::Date => DataType::Date,
            DataTypeFile::DateTime => DataType::DateTime,
            DataTypeFile::Duration => DataType::Duration,
        }
    }
}

impl From<DataType> for DataTypeFile {
    fn from(data_type: DataType) -> Self {
        match data_type {
            DataType::Int => DataTypeFile::Int,
            DataType::Float => DataTypeFile::Float,
            DataType::Decimal => DataTypeFile::Decimal,
            DataType::Bool => DataTypeFile::Bool,
            DataType::Text => DataTypeFile::Text,
            DataType::Date => DataTypeFile::Date,
            DataType::DateTime => DataTypeFile::DateTime,
            DataType::Duration => DataTypeFile::Duration,
        }
    }
}

impl From<DirectionFile> for Direction {
    fn from(direction: DirectionFile) -> Self {
        match direction {
            DirectionFile::Asc => Direction::Asc,
            DirectionFile::Desc => Direction::Desc,
        }
    }
}

impl From<Direction> for DirectionFile {
    fn from(direction: Direction) -> Self {
        match direction {
            Direction::Asc => DirectionFile::Asc,
            Direction::Desc => DirectionFile::Desc,
        }
    }
}

impl From<JoinKindFile> for JoinKind {
    fn from(kind: JoinKindFile) -> Self {
        match kind {
            JoinKindFile::Inner => JoinKind::Inner,
            JoinKindFile::Left => JoinKind::Left,
            JoinKindFile::Right => JoinKind::Right,
            JoinKindFile::Full => JoinKind::Full,
        }
    }
}

impl From<JoinKind> for JoinKindFile {
    fn from(kind: JoinKind) -> Self {
        match kind {
            JoinKind::Inner => JoinKindFile::Inner,
            JoinKind::Left => JoinKindFile::Left,
            JoinKind::Right => JoinKindFile::Right,
            JoinKind::Full => JoinKindFile::Full,
        }
    }
}
