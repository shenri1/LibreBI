//! Modelo de uma consulta do LibreQuery.
//!
//! Uma consulta é uma **fonte** mais uma **lista de passos**. Estes tipos apenas
//! descrevem a intenção do usuário; a tradução para SQL fica em [`crate::compile`].

/// Direção de ordenação de uma coluna.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Asc,
    Desc,
}

/// Tipos de coluna aceitos pelo modelo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Int,
    Float,
    Decimal,
    Bool,
    Text,
    Date,
    DateTime,
    Duration,
}

/// Função de agregação usada em um passo de agrupamento.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggFunc {
    Sum,
    Avg,
    Min,
    Max,
    Count,
    CountDistinct,
}

/// Tipo de junção entre a consulta atual e outra fonte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
    Full,
}

/// Uma coluna calculada por agregação.
///
/// `column` é `None` apenas para [`AggFunc::Count`], que vira `COUNT(*)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Aggregation {
    /// Nome da coluna resultante.
    pub name: String,
    /// Função aplicada.
    pub func: AggFunc,
    /// Coluna de entrada; `None` significa `*`.
    pub column: Option<String>,
}

/// Um passo de transformação da consulta.
///
/// Cada passo é aplicado sobre o resultado do passo anterior e vira um CTE.
/// Expressões (`Filter`, `AddColumn::expr` e `RawSql`) são texto SQL livre e
/// entram no SQL gerado sem citação; nomes de coluna são identificadores.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Mantém apenas as colunas indicadas.
    SelectColumns(Vec<String>),
    /// Remove as colunas indicadas.
    RemoveColumns(Vec<String>),
    /// Renomeia colunas como pares `(antigo, novo)`.
    RenameColumns(Vec<(String, String)>),
    /// Converte a coluna para outro tipo.
    ChangeType { column: String, to: DataType },
    /// Filtra linhas com uma expressão SQL (usada em `WHERE`).
    Filter(String),
    /// Acrescenta uma coluna calculada por uma expressão SQL.
    AddColumn { name: String, expr: String },
    /// Ordena o resultado.
    Sort(Vec<(String, Direction)>),
    /// Agrupa por chaves e calcula agregações.
    GroupBy {
        keys: Vec<String>,
        aggregations: Vec<Aggregation>,
    },
    /// Junta com outra fonte por igualdade de chaves.
    Join {
        kind: JoinKind,
        other: String,
        left_key: String,
        right_key: String,
    },
    /// Empilha o resultado de outra fonte (`UNION ALL`).
    Append { other: String },
    /// Remove linhas duplicadas.
    Distinct,
    /// Substitui valores de texto de uma coluna.
    ReplaceValues {
        column: String,
        from: String,
        to: String,
    },
    /// Consulta SQL livre; o nome `prev` refere-se ao passo anterior.
    RawSql(String),
}

/// Uma consulta: fonte inicial e passos aplicados em sequência.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    /// Identificador da consulta.
    pub id: String,
    /// Nome da fonte (tabela ou consulta) de onde os dados vêm.
    pub source: String,
    /// Passos na ordem de execução; pode ser vazio.
    pub steps: Vec<Step>,
}
