//! Ajudantes compartilhados pelos testes de integração.

#![allow(dead_code)]

use std::fs;
use std::path::Path;

use libreproject::{FORMAT_VERSION, Project, SourceDef, SourceKind};
use librequery::{AggFunc, Aggregation, DataType, Direction, JoinKind, Query, Step};

/// Lista de nomes de coluna.
pub fn columns(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

/// Agregação de um passo `group_by`.
pub fn aggregation(name: &str, func: AggFunc, column: Option<&str>) -> Aggregation {
    Aggregation {
        name: name.to_string(),
        func,
        column: column.map(|column| column.to_string()),
    }
}

/// Fonte de dados.
pub fn source(id: &str, kind: SourceKind, path: &str) -> SourceDef {
    SourceDef {
        id: id.to_string(),
        kind,
        path: path.to_string(),
    }
}

/// Consulta.
pub fn query(id: &str, source: &str, steps: Vec<Step>) -> Query {
    Query {
        id: id.to_string(),
        source: source.to_string(),
        steps,
    }
}

/// Projeto na versão de formato atual.
pub fn project(name: &str, sources: Vec<SourceDef>, queries: Vec<Query>) -> Project {
    Project {
        name: name.to_string(),
        format_version: FORMAT_VERSION,
        sources,
        queries,
    }
}

/// Projeto do exemplo da seção 6 da especificação, mais a consulta agregada.
pub fn example_project() -> Project {
    project(
        "Vendas",
        vec![
            source("arquivo_vendas", SourceKind::Csv, "data/vendas.csv"),
            source("produtos", SourceKind::Csv, "data/produtos.csv"),
        ],
        vec![
            query(
                "receita_por_categoria",
                "vendas",
                vec![
                    Step::Join {
                        kind: JoinKind::Left,
                        other: "produtos".to_string(),
                        left_key: "produto_id".to_string(),
                        right_key: "produto_id".to_string(),
                    },
                    Step::GroupBy {
                        keys: columns(&["categoria"]),
                        aggregations: vec![aggregation("receita", AggFunc::Sum, Some("valor"))],
                    },
                    Step::Sort(vec![("categoria".to_string(), Direction::Asc)]),
                ],
            ),
            query(
                "vendas",
                "arquivo_vendas",
                vec![
                    Step::SelectColumns(columns(&["data", "produto_id", "qtd", "preco"])),
                    Step::ChangeType {
                        column: "data".to_string(),
                        to: DataType::Date,
                    },
                    Step::AddColumn {
                        name: "valor".to_string(),
                        expr: "qtd * preco".to_string(),
                    },
                    Step::Filter("qtd > 0".to_string()),
                ],
            ),
        ],
    )
}

/// Projeto com todos os tipos de passo em uma consulta.
pub fn all_steps_project() -> Project {
    project(
        "Todos os passos",
        vec![
            source("arquivo_vendas", SourceKind::Csv, "data/vendas.csv"),
            source("produtos", SourceKind::Parquet, "data/produtos.parquet"),
        ],
        vec![
            query("historico", "arquivo_vendas", vec![]),
            query(
                "vendas_completa",
                "arquivo_vendas",
                vec![
                    Step::SelectColumns(columns(&["data", "produto_id", "qtd", "preco"])),
                    Step::RemoveColumns(columns(&["preco"])),
                    Step::RenameColumns(vec![("qtd".to_string(), "quantidade".to_string())]),
                    Step::ChangeType {
                        column: "data".to_string(),
                        to: DataType::Date,
                    },
                    Step::Filter("quantidade > 0".to_string()),
                    Step::AddColumn {
                        name: "valor".to_string(),
                        expr: "quantidade * preco".to_string(),
                    },
                    Step::Sort(vec![
                        ("valor".to_string(), Direction::Desc),
                        ("data".to_string(), Direction::Asc),
                    ]),
                    Step::GroupBy {
                        keys: columns(&["categoria"]),
                        aggregations: vec![
                            aggregation("total", AggFunc::Sum, Some("valor")),
                            aggregation("media", AggFunc::Avg, Some("valor")),
                            aggregation("minimo", AggFunc::Min, Some("valor")),
                            aggregation("maximo", AggFunc::Max, Some("valor")),
                            aggregation("linhas", AggFunc::Count, None),
                            aggregation("datas", AggFunc::CountDistinct, Some("data")),
                        ],
                    },
                    Step::Join {
                        kind: JoinKind::Left,
                        other: "produtos".to_string(),
                        left_key: "produto_id".to_string(),
                        right_key: "produto_id".to_string(),
                    },
                    Step::Append {
                        other: "historico".to_string(),
                    },
                    Step::Distinct,
                    Step::ReplaceValues {
                        column: "categoria".to_string(),
                        from: "Casa".to_string(),
                        to: "Lar".to_string(),
                    },
                    Step::RawSql("SELECT * FROM prev".to_string()),
                ],
            ),
        ],
    )
}

/// Projeto que exercita todos os tipos de dado e os quatro tipos de junção.
pub fn variations_project() -> Project {
    project(
        "Variações",
        vec![
            source("base", SourceKind::Csv, "data/base.csv"),
            source("outra", SourceKind::Csv, "data/outra.csv"),
        ],
        vec![query(
            "variacoes",
            "base",
            vec![
                Step::ChangeType {
                    column: "coluna_int".to_string(),
                    to: DataType::Int,
                },
                Step::ChangeType {
                    column: "coluna_float".to_string(),
                    to: DataType::Float,
                },
                Step::ChangeType {
                    column: "coluna_decimal".to_string(),
                    to: DataType::Decimal,
                },
                Step::ChangeType {
                    column: "coluna_bool".to_string(),
                    to: DataType::Bool,
                },
                Step::ChangeType {
                    column: "coluna_text".to_string(),
                    to: DataType::Text,
                },
                Step::ChangeType {
                    column: "coluna_date".to_string(),
                    to: DataType::Date,
                },
                Step::ChangeType {
                    column: "coluna_datetime".to_string(),
                    to: DataType::DateTime,
                },
                Step::ChangeType {
                    column: "coluna_duration".to_string(),
                    to: DataType::Duration,
                },
                Step::Join {
                    kind: JoinKind::Inner,
                    other: "outra".to_string(),
                    left_key: "coluna_int".to_string(),
                    right_key: "coluna_int".to_string(),
                },
                Step::Join {
                    kind: JoinKind::Right,
                    other: "outra".to_string(),
                    left_key: "coluna_int".to_string(),
                    right_key: "coluna_int".to_string(),
                },
                Step::Join {
                    kind: JoinKind::Full,
                    other: "outra".to_string(),
                    left_key: "coluna_int".to_string(),
                    right_key: "coluna_int".to_string(),
                },
            ],
        )],
    )
}

/// Texto canônico esperado de `queries/vendas.yaml` do exemplo da seção 6.
pub const CANONICAL_QUERY: &str = concat!(
    "id: vendas\n",
    "source: arquivo_vendas\n",
    "steps:\n",
    "- op: select_columns\n",
    "  columns:\n",
    "  - data\n",
    "  - produto_id\n",
    "  - qtd\n",
    "  - preco\n",
    "- op: change_type\n",
    "  column: data\n",
    "  to: date\n",
    "- op: add_column\n",
    "  name: valor\n",
    "  expr: qtd * preco\n",
    "- op: filter\n",
    "  expr: qtd > 0\n",
);

/// Lê todos os arquivos da pasta como pares `(caminho relativo, conteúdo)`.
pub fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files = Vec::new();
    collect(dir, dir, &mut files);
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, Vec<u8>)>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(root, &path, files);
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        files.push((relative, fs::read(&path).unwrap()));
    }
}
