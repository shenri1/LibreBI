use librequery::{
    AggFunc, Aggregation, CompileError, DataType, Direction, JoinKind, Query, Step, compile,
};

fn query(source: &str, steps: Vec<Step>) -> Query {
    Query {
        id: "consulta".to_string(),
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

/// Exemplo da seção 6 da especificação: quatro passos sobre `src_arquivo_vendas`.
#[test]
fn exemplo_da_especificacao_gera_cadeia_de_ctes() {
    let query = query(
        "src_arquivo_vendas",
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
    );

    let expected = "\
WITH s0 AS (SELECT * FROM \"src_arquivo_vendas\"),
     s1 AS (SELECT \"data\", \"produto_id\", \"qtd\", \"preco\" FROM s0),
     s2 AS (SELECT * EXCEPT (\"data\"), CAST(\"data\" AS DATE) AS \"data\" FROM s1),
     s3 AS (SELECT *, qtd * preco AS \"valor\" FROM s2)
SELECT * FROM s3 WHERE qtd > 0";

    assert_eq!(compile(&query).unwrap(), expected);
}

#[test]
fn consulta_sem_passos_gera_apenas_s0() {
    let query = query("src_arquivo_vendas", vec![]);

    let expected = "\
WITH s0 AS (SELECT * FROM \"src_arquivo_vendas\")
SELECT * FROM s0";

    assert_eq!(compile(&query).unwrap(), expected);
}

#[test]
fn remove_columns_usa_except() {
    let query = query(
        "vendas",
        vec![Step::RemoveColumns(columns(&["senha", "nota"]))],
    );

    let expected = "\
WITH s0 AS (SELECT * FROM \"vendas\")
SELECT * EXCEPT (\"senha\", \"nota\") FROM s0";

    assert_eq!(compile(&query).unwrap(), expected);
}

#[test]
fn rename_columns_troca_cada_coluna() {
    let query = query(
        "vendas",
        vec![Step::RenameColumns(vec![
            ("data".to_string(), "data_venda".to_string()),
            ("qtd".to_string(), "quantidade".to_string()),
        ])],
    );

    let expected = "\
WITH s0 AS (SELECT * FROM \"vendas\")
SELECT * EXCEPT (\"data\", \"qtd\"), \"data\" AS \"data_venda\", \"qtd\" AS \"quantidade\" FROM s0";

    assert_eq!(compile(&query).unwrap(), expected);
}

#[test]
fn change_type_mapeia_todos_os_tipos() {
    let cases = [
        (DataType::Int, "BIGINT"),
        (DataType::Float, "DOUBLE"),
        (DataType::Decimal, "DECIMAL(38,9)"),
        (DataType::Bool, "BOOLEAN"),
        (DataType::Text, "VARCHAR"),
        (DataType::Date, "DATE"),
        (DataType::DateTime, "TIMESTAMP"),
        (DataType::Duration, "INTERVAL"),
    ];

    for (data_type, sql_type) in cases {
        let query = query(
            "tabela",
            vec![Step::ChangeType {
                column: "coluna".to_string(),
                to: data_type,
            }],
        );
        let expected = format!(
            "WITH s0 AS (SELECT * FROM \"tabela\")\n\
             SELECT * EXCEPT (\"coluna\"), CAST(\"coluna\" AS {sql_type}) AS \"coluna\" FROM s0"
        );

        assert_eq!(compile(&query).unwrap(), expected, "tipo {sql_type}");
    }
}

#[test]
fn sort_gera_order_by_com_direcoes() {
    let query = query(
        "vendas",
        vec![Step::Sort(vec![
            ("data".to_string(), Direction::Desc),
            ("produto".to_string(), Direction::Asc),
        ])],
    );

    let expected = "\
WITH s0 AS (SELECT * FROM \"vendas\")
SELECT * FROM s0 ORDER BY \"data\" DESC, \"produto\" ASC";

    assert_eq!(compile(&query).unwrap(), expected);
}

#[test]
fn group_by_com_chaves_e_todas_as_agregacoes() {
    let query = query(
        "vendas",
        vec![Step::GroupBy {
            keys: columns(&["produto_id"]),
            aggregations: vec![
                aggregation("total", AggFunc::Sum, Some("valor")),
                aggregation("media", AggFunc::Avg, Some("valor")),
                aggregation("minimo", AggFunc::Min, Some("valor")),
                aggregation("maximo", AggFunc::Max, Some("valor")),
                aggregation("pedidos", AggFunc::CountDistinct, Some("pedido_id")),
                aggregation("linhas", AggFunc::Count, None),
            ],
        }],
    );

    let expected = "\
WITH s0 AS (SELECT * FROM \"vendas\")
SELECT \"produto_id\", SUM(\"valor\") AS \"total\", AVG(\"valor\") AS \"media\", MIN(\"valor\") AS \"minimo\", MAX(\"valor\") AS \"maximo\", COUNT(DISTINCT \"pedido_id\") AS \"pedidos\", COUNT(*) AS \"linhas\" FROM s0 GROUP BY \"produto_id\"";

    assert_eq!(compile(&query).unwrap(), expected);
}

#[test]
fn group_by_apenas_com_chaves_e_permitido() {
    let query = query(
        "vendas",
        vec![Step::GroupBy {
            keys: columns(&["produto_id"]),
            aggregations: vec![],
        }],
    );

    let expected = "\
WITH s0 AS (SELECT * FROM \"vendas\")
SELECT \"produto_id\" FROM s0 GROUP BY \"produto_id\"";

    assert_eq!(compile(&query).unwrap(), expected);
}

#[test]
fn group_by_sem_chaves_agrega_a_tabela_inteira() {
    let query = query(
        "vendas",
        vec![Step::GroupBy {
            keys: vec![],
            aggregations: vec![aggregation("media", AggFunc::Avg, Some("valor"))],
        }],
    );

    let expected = "\
WITH s0 AS (SELECT * FROM \"vendas\")
SELECT AVG(\"valor\") AS \"media\" FROM s0";

    assert_eq!(compile(&query).unwrap(), expected);
}

#[test]
fn join_gera_cada_tipo_de_juncao() {
    let cases = [
        (JoinKind::Inner, "INNER JOIN"),
        (JoinKind::Left, "LEFT JOIN"),
        (JoinKind::Right, "RIGHT JOIN"),
        (JoinKind::Full, "FULL OUTER JOIN"),
    ];

    for (kind, keyword) in cases {
        let query = query(
            "vendas",
            vec![Step::Join {
                kind,
                other: "produtos".to_string(),
                left_key: "produto_id".to_string(),
                right_key: "id".to_string(),
            }],
        );
        let expected = format!(
            "WITH s0 AS (SELECT * FROM \"vendas\")\n\
             SELECT * FROM s0 {keyword} \"produtos\" ON s0.\"produto_id\" = \"produtos\".\"id\""
        );

        assert_eq!(compile(&query).unwrap(), expected, "junção {keyword}");
    }
}

#[test]
fn append_gera_union_all() {
    let query = query(
        "vendas_2024",
        vec![Step::Append {
            other: "vendas_2025".to_string(),
        }],
    );

    let expected = "\
WITH s0 AS (SELECT * FROM \"vendas_2024\")
SELECT * FROM s0 UNION ALL SELECT * FROM \"vendas_2025\"";

    assert_eq!(compile(&query).unwrap(), expected);
}

#[test]
fn distinct_gera_select_distinct() {
    let query = query("vendas", vec![Step::Distinct]);

    let expected = "\
WITH s0 AS (SELECT * FROM \"vendas\")
SELECT DISTINCT * FROM s0";

    assert_eq!(compile(&query).unwrap(), expected);
}

#[test]
fn replace_values_usa_case_e_escapa_apostrofo() {
    let query = query(
        "vendas",
        vec![Step::ReplaceValues {
            column: "estado".to_string(),
            from: "Solteiro".to_string(),
            to: "O'Brien".to_string(),
        }],
    );

    let expected = "\
WITH s0 AS (SELECT * FROM \"vendas\")
SELECT * EXCEPT (\"estado\"), CASE WHEN \"estado\" = 'Solteiro' THEN 'O''Brien' ELSE \"estado\" END AS \"estado\" FROM s0";

    assert_eq!(compile(&query).unwrap(), expected);
}

#[test]
fn raw_sql_troca_prev_pelo_passo_anterior() {
    let com_prev = query(
        "vendas",
        vec![Step::RawSql(
            "SELECT a, prev.b FROM prev WHERE prev.a > 1 AND 'prev' = 'prev'".to_string(),
        )],
    );

    let expected = "\
WITH s0 AS (SELECT * FROM \"vendas\")
SELECT a, s0.b FROM s0 WHERE s0.a > 1 AND 'prev' = 'prev'";

    assert_eq!(compile(&com_prev).unwrap(), expected);

    // `previous` não é o identificador `prev` e não deve ser trocado.
    let outra = query(
        "vendas",
        vec![Step::RawSql("SELECT previous FROM prev".to_string())],
    );
    let expected = "\
WITH s0 AS (SELECT * FROM \"vendas\")
SELECT previous FROM s0";
    assert_eq!(compile(&outra).unwrap(), expected);
}

#[test]
fn escapa_aspas_em_identificadores() {
    let query = query(
        "ve\"ndas",
        vec![Step::SelectColumns(columns(&["col\"una"]))],
    );

    let expected = "\
WITH s0 AS (SELECT * FROM \"ve\"\"ndas\")
SELECT \"col\"\"una\" FROM s0";

    assert_eq!(compile(&query).unwrap(), expected);
}

#[test]
fn utilitarios_de_citacao() {
    assert_eq!(librequery::sql::quote_ident("vendas"), "\"vendas\"");
    assert_eq!(librequery::sql::quote_ident("a\"b"), "\"a\"\"b\"");
    assert_eq!(librequery::sql::quote_literal("texto"), "'texto'");
    assert_eq!(librequery::sql::quote_literal("O'Brien"), "'O''Brien'");
}

#[test]
fn erro_para_fonte_vazia() {
    assert_eq!(compile(&query("", vec![])), Err(CompileError::EmptySource));
    assert_eq!(
        compile(&query("   ", vec![])),
        Err(CompileError::EmptySource)
    );
}

#[test]
fn erro_para_lista_de_colunas_vazia() {
    assert_eq!(
        compile(&query("vendas", vec![Step::SelectColumns(vec![])])),
        Err(CompileError::EmptyColumns)
    );
    assert_eq!(
        compile(&query("vendas", vec![Step::RemoveColumns(vec![])])),
        Err(CompileError::EmptyColumns)
    );
}

#[test]
fn erro_para_group_by_sem_chaves_e_sem_agregacoes() {
    assert_eq!(
        compile(&query(
            "vendas",
            vec![Step::GroupBy {
                keys: vec![],
                aggregations: vec![],
            }]
        )),
        Err(CompileError::EmptyGroupBy)
    );
}

#[test]
fn erro_para_identificador_vazio() {
    let cases = [
        Step::SelectColumns(columns(&[""])),
        Step::RemoveColumns(columns(&["  "])),
        Step::RenameColumns(vec![("data".to_string(), "".to_string())]),
        Step::ChangeType {
            column: "".to_string(),
            to: DataType::Date,
        },
        Step::AddColumn {
            name: "".to_string(),
            expr: "1".to_string(),
        },
        Step::Sort(vec![("".to_string(), Direction::Asc)]),
        Step::GroupBy {
            keys: columns(&[""]),
            aggregations: vec![],
        },
        Step::Join {
            kind: JoinKind::Inner,
            other: "".to_string(),
            left_key: "a".to_string(),
            right_key: "b".to_string(),
        },
        Step::Append {
            other: "".to_string(),
        },
        Step::ReplaceValues {
            column: "".to_string(),
            from: "a".to_string(),
            to: "b".to_string(),
        },
    ];

    for step in cases {
        assert_eq!(
            compile(&query("vendas", vec![step.clone()])),
            Err(CompileError::EmptyIdentifier),
            "passo {step:?}"
        );
    }

    // Agregação sem nome, coluna ausente em função que não seja COUNT e
    // COUNT(DISTINCT) sem coluna também são erros.
    let aggregations = [
        aggregation("", AggFunc::Sum, Some("valor")),
        aggregation("total", AggFunc::Sum, None),
        aggregation("distintos", AggFunc::CountDistinct, None),
    ];
    for aggregation in aggregations {
        assert_eq!(
            compile(&query(
                "vendas",
                vec![Step::GroupBy {
                    keys: columns(&["produto_id"]),
                    aggregations: vec![aggregation],
                }]
            )),
            Err(CompileError::EmptyIdentifier)
        );
    }
}
