mod common;

use std::path::PathBuf;

use libreengine::{Engine, format_batches};
use libreproject::Project;
use tempfile::TempDir;

/// Pasta do projeto de exemplo na raiz do repositório.
fn example_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/vendas")
}

#[test]
fn exemplo_esta_na_forma_canonica() {
    let dir = example_dir();
    let project = Project::load(&dir).unwrap();

    let temp = TempDir::new().unwrap();
    project.save(temp.path()).unwrap();

    let expected: Vec<(String, Vec<u8>)> = common::snapshot(&dir)
        .into_iter()
        .filter(|(path, _)| path.ends_with(".yaml"))
        .collect();
    assert_eq!(common::snapshot(temp.path()), expected);
}

#[tokio::test]
async fn exemplo_abre_e_executa_as_consultas() {
    let dir = example_dir();
    let project = Project::load(&dir).unwrap();
    project.validate().unwrap();

    let engine = Engine::new();
    project.open_in(&engine, &dir).await.unwrap();

    let vendas = project
        .queries
        .iter()
        .find(|query| query.id == "vendas")
        .unwrap();
    let expected_vendas = "\
produto_id | qtd | preco | data | valor
1 | 3 | 10.5 | 2026-01-05 | 31.5
2 | 2 | 25.0 | 2026-01-06 | 50.0
3 | 5 | 4.0 | 2026-01-12 | 20.0
2 | 1 | 25.0 | 2026-01-13 | 25.0
4 | 4 | 7.25 | 2026-01-14 | 29.0
1 | 2 | 10.5 | 2026-01-15 | 21.0
3 | 6 | 4.0 | 2026-01-16 | 24.0";
    assert_eq!(
        format_batches(&engine.run(vendas).await.unwrap()),
        expected_vendas
    );

    let receita = project
        .queries
        .iter()
        .find(|query| query.id == "receita_por_categoria")
        .unwrap();
    let expected_receita = "\
categoria | receita
Casa | 44.0
Cozinha | 29.0
Papelaria | 127.5";
    assert_eq!(
        format_batches(&engine.run(receita).await.unwrap()),
        expected_receita
    );
}
