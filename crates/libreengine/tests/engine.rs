use std::path::PathBuf;

use libreengine::{Engine, EngineError, format_batches};
use librequery::{AggFunc, Aggregation, DataType, Direction, JoinKind, Query, Step};

/// Caminho de um arquivo da pasta de dados de teste.
fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data")
        .join(name)
}

/// Motor com `vendas` e `produtos` registrados.
async fn engine() -> Engine {
    let engine = Engine::new();
    engine
        .register_csv("vendas", &data("vendas.csv"))
        .await
        .unwrap();
    engine
        .register_csv("produtos", &data("produtos.csv"))
        .await
        .unwrap();
    engine
}

fn query(id: &str, source: &str, steps: Vec<Step>) -> Query {
    Query {
        id: id.to_string(),
        source: source.to_string(),
        steps,
    }
}

fn columns(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

fn aggregation(name: &str, func: AggFunc, column: Option<&str>) -> Aggregation {
    Aggregation {
        name: name.to_string(),
        func,
        column: column.map(|column| column.to_string()),
    }
}

/// Exemplo da seção 6 da especificação sobre `vendas.csv`.
fn consulta_do_exemplo() -> Query {
    query(
        "vendas",
        "vendas",
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
    )
}

#[tokio::test]
async fn exemplo_da_especificacao() {
    let engine = engine().await;

    // `change_type` recria a coluna no fim da projeção; por isso `data` aparece
    // depois de `preco`.
    let expected = "\
produto_id | qtd | preco | data | valor
1 | 3 | 10.5 | 2026-01-05 | 31.5
2 | 2 | 25.0 | 2026-01-06 | 50.0
3 | 5 | 4.0 | 2026-01-12 | 20.0
2 | 1 | 25.0 | 2026-01-13 | 25.0
4 | 4 | 7.25 | 2026-01-14 | 29.0
1 | 2 | 10.5 | 2026-01-15 | 21.0
3 | 6 | 4.0 | 2026-01-16 | 24.0";

    let batches = engine.run(&consulta_do_exemplo()).await.unwrap();
    assert_eq!(format_batches(&batches), expected);
}

#[tokio::test]
async fn remove_columns() {
    let engine = engine().await;
    let consulta = query(
        "vendas",
        "vendas",
        vec![Step::RemoveColumns(columns(&["preco", "produto_id"]))],
    );

    let expected = "\
data | qtd
2026-01-05 | 3
2026-01-06 | 2
2026-01-07 | 0
2026-01-12 | 5
2026-01-13 | 1
2026-01-14 | 4
2026-01-15 | 2
2026-01-16 | 6";

    let batches = engine.run(&consulta).await.unwrap();
    assert_eq!(format_batches(&batches), expected);
}

#[tokio::test]
async fn rename_columns() {
    let engine = engine().await;
    let consulta = query(
        "vendas",
        "vendas",
        vec![
            Step::RenameColumns(vec![("qtd".to_string(), "quantidade".to_string())]),
            Step::SelectColumns(columns(&["data", "quantidade"])),
        ],
    );

    let expected = "\
data | quantidade
2026-01-05 | 3
2026-01-06 | 2
2026-01-07 | 0
2026-01-12 | 5
2026-01-13 | 1
2026-01-14 | 4
2026-01-15 | 2
2026-01-16 | 6";

    let batches = engine.run(&consulta).await.unwrap();
    assert_eq!(format_batches(&batches), expected);
}

#[tokio::test]
async fn sort() {
    let engine = engine().await;
    let consulta = query(
        "vendas",
        "vendas",
        vec![
            Step::SelectColumns(columns(&["data", "qtd"])),
            Step::Sort(vec![
                ("qtd".to_string(), Direction::Desc),
                ("data".to_string(), Direction::Asc),
            ]),
        ],
    );

    let expected = "\
data | qtd
2026-01-16 | 6
2026-01-12 | 5
2026-01-14 | 4
2026-01-05 | 3
2026-01-06 | 2
2026-01-15 | 2
2026-01-13 | 1
2026-01-07 | 0";

    let batches = engine.run(&consulta).await.unwrap();
    assert_eq!(format_batches(&batches), expected);
}

#[tokio::test]
async fn group_by_com_soma_e_contagem() {
    let engine = engine().await;
    let consulta = query(
        "vendas",
        "vendas",
        vec![
            Step::AddColumn {
                name: "valor".to_string(),
                expr: "qtd * preco".to_string(),
            },
            Step::GroupBy {
                keys: columns(&["produto_id"]),
                aggregations: vec![
                    aggregation("total", AggFunc::Sum, Some("valor")),
                    aggregation("linhas", AggFunc::Count, None),
                ],
            },
            Step::Sort(vec![("produto_id".to_string(), Direction::Asc)]),
        ],
    );

    let expected = "\
produto_id | total | linhas
1 | 52.5 | 3
2 | 75.0 | 2
3 | 44.0 | 2
4 | 29.0 | 1";

    let batches = engine.run(&consulta).await.unwrap();
    assert_eq!(format_batches(&batches), expected);
}

#[tokio::test]
async fn join_com_produtos() {
    let engine = engine().await;
    let consulta = query(
        "vendas",
        "vendas",
        vec![
            Step::Join {
                kind: JoinKind::Inner,
                other: "produtos".to_string(),
                left_key: "produto_id".to_string(),
                right_key: "produto_id".to_string(),
            },
            Step::SelectColumns(columns(&["data", "nome", "qtd"])),
            Step::Sort(vec![("data".to_string(), Direction::Asc)]),
        ],
    );

    let expected = "\
data | nome | qtd
2026-01-05 | Caneta | 3
2026-01-06 | Caderno | 2
2026-01-07 | Caneta | 0
2026-01-12 | Luminaria | 5
2026-01-13 | Caderno | 1
2026-01-14 | Copo | 4
2026-01-15 | Caneta | 2
2026-01-16 | Luminaria | 6";

    let batches = engine.run(&consulta).await.unwrap();
    assert_eq!(format_batches(&batches), expected);
}

#[tokio::test]
async fn append_empilha_outra_fonte() {
    let engine = engine().await;
    let consulta = query(
        "vendas",
        "vendas",
        vec![Step::Append {
            other: "vendas".to_string(),
        }],
    );

    let expected = "\
data | produto_id | qtd | preco
2026-01-05 | 1 | 3 | 10.5
2026-01-06 | 2 | 2 | 25.0
2026-01-07 | 1 | 0 | 10.5
2026-01-12 | 3 | 5 | 4.0
2026-01-13 | 2 | 1 | 25.0
2026-01-14 | 4 | 4 | 7.25
2026-01-15 | 1 | 2 | 10.5
2026-01-16 | 3 | 6 | 4.0
2026-01-05 | 1 | 3 | 10.5
2026-01-06 | 2 | 2 | 25.0
2026-01-07 | 1 | 0 | 10.5
2026-01-12 | 3 | 5 | 4.0
2026-01-13 | 2 | 1 | 25.0
2026-01-14 | 4 | 4 | 7.25
2026-01-15 | 1 | 2 | 10.5
2026-01-16 | 3 | 6 | 4.0";

    let batches = engine.run(&consulta).await.unwrap();
    assert_eq!(format_batches(&batches), expected);
}

#[tokio::test]
async fn distinct() {
    let engine = engine().await;
    let consulta = query(
        "vendas",
        "vendas",
        vec![
            Step::SelectColumns(columns(&["produto_id"])),
            Step::Distinct,
            Step::Sort(vec![("produto_id".to_string(), Direction::Asc)]),
        ],
    );

    let expected = "\
produto_id
1
2
3
4";

    let batches = engine.run(&consulta).await.unwrap();
    assert_eq!(format_batches(&batches), expected);
}

#[tokio::test]
async fn replace_values() {
    let engine = engine().await;
    let consulta = query(
        "produtos",
        "produtos",
        vec![
            Step::ReplaceValues {
                column: "categoria".to_string(),
                from: "Papelaria".to_string(),
                to: "Escritorio".to_string(),
            },
            Step::SelectColumns(columns(&["nome", "categoria"])),
            Step::Sort(vec![("nome".to_string(), Direction::Asc)]),
        ],
    );

    let expected = "\
nome | categoria
Caderno | Escritorio
Caneta | Escritorio
Copo | Cozinha
Luminaria | Casa";

    let batches = engine.run(&consulta).await.unwrap();
    assert_eq!(format_batches(&batches), expected);
}

#[tokio::test]
async fn raw_sql_usa_prev() {
    let engine = engine().await;
    let consulta = query(
        "vendas",
        "vendas",
        vec![Step::RawSql(
            "SELECT produto_id, qtd * preco AS total FROM prev WHERE qtd > 0 ORDER BY total DESC"
                .to_string(),
        )],
    );

    let expected = "\
produto_id | total
2 | 50.0
1 | 31.5
4 | 29.0
2 | 25.0
3 | 24.0
1 | 21.0
3 | 20.0";

    let batches = engine.run(&consulta).await.unwrap();
    assert_eq!(format_batches(&batches), expected);
}

#[tokio::test]
async fn preview_limita_linhas() {
    let engine = engine().await;

    let expected = "\
produto_id | qtd | preco | data | valor
1 | 3 | 10.5 | 2026-01-05 | 31.5
2 | 2 | 25.0 | 2026-01-06 | 50.0
3 | 5 | 4.0 | 2026-01-12 | 20.0";

    let batches = engine.preview(&consulta_do_exemplo(), 3).await.unwrap();
    assert_eq!(format_batches(&batches), expected);
}

#[tokio::test]
async fn preview_until_step() {
    let engine = engine().await;
    let consulta = query(
        "vendas",
        "vendas",
        vec![
            Step::Filter("qtd > 0".to_string()),
            Step::AddColumn {
                name: "valor".to_string(),
                expr: "qtd * preco".to_string(),
            },
            Step::Sort(vec![("valor".to_string(), Direction::Desc)]),
        ],
    );

    let sem_valor = "\
data | produto_id | qtd | preco
2026-01-05 | 1 | 3 | 10.5
2026-01-06 | 2 | 2 | 25.0
2026-01-12 | 3 | 5 | 4.0
2026-01-13 | 2 | 1 | 25.0
2026-01-14 | 4 | 4 | 7.25
2026-01-15 | 1 | 2 | 10.5
2026-01-16 | 3 | 6 | 4.0";
    let batches = engine.preview_until_step(&consulta, 0, 10).await.unwrap();
    assert_eq!(format_batches(&batches), sem_valor);

    let com_valor = "\
data | produto_id | qtd | preco | valor
2026-01-05 | 1 | 3 | 10.5 | 31.5
2026-01-06 | 2 | 2 | 25.0 | 50.0";
    let batches = engine.preview_until_step(&consulta, 1, 2).await.unwrap();
    assert_eq!(format_batches(&batches), com_valor);

    let ordenado = "\
data | produto_id | qtd | preco | valor
2026-01-06 | 2 | 2 | 25.0 | 50.0
2026-01-05 | 1 | 3 | 10.5 | 31.5
2026-01-14 | 4 | 4 | 7.25 | 29.0";
    let batches = engine.preview_until_step(&consulta, 2, 3).await.unwrap();
    assert_eq!(format_batches(&batches), ordenado);
}

#[tokio::test]
async fn preview_until_step_com_indice_invalido() {
    let engine = engine().await;
    let consulta = consulta_do_exemplo();

    let error = engine
        .preview_until_step(&consulta, 4, 10)
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::StepOutOfRange { index: 4, steps: 4 }
    ));
    assert_eq!(
        error.to_string(),
        "passo 4 não existe: a consulta tem 4 passo(s)"
    );

    let sem_passos = query("vendas", "vendas", vec![]);
    assert!(matches!(
        engine.preview_until_step(&sem_passos, 0, 10).await,
        Err(EngineError::StepOutOfRange { index: 0, steps: 0 })
    ));
}

#[tokio::test]
async fn consulta_usa_outra_consulta_como_fonte() {
    let engine = engine().await;

    let vendas_caras = query(
        "vendas_caras",
        "vendas",
        vec![
            Step::AddColumn {
                name: "valor".to_string(),
                expr: "qtd * preco".to_string(),
            },
            Step::Filter("valor > 20".to_string()),
        ],
    );
    engine.register_query(&vendas_caras).await.unwrap();

    let resumo = query(
        "resumo",
        "vendas_caras",
        vec![
            Step::GroupBy {
                keys: columns(&["produto_id"]),
                aggregations: vec![
                    aggregation("total", AggFunc::Sum, Some("valor")),
                    aggregation("linhas", AggFunc::Count, None),
                ],
            },
            Step::Sort(vec![("produto_id".to_string(), Direction::Asc)]),
        ],
    );

    let expected = "\
produto_id | total | linhas
1 | 52.5 | 2
2 | 75.0 | 2
3 | 24.0 | 1
4 | 29.0 | 1";

    let batches = engine.run(&resumo).await.unwrap();
    assert_eq!(format_batches(&batches), expected);
}
