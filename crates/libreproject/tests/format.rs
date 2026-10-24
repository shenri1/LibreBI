mod common;

use std::fs;

use libreproject::{Project, SourceKind};
use tempfile::TempDir;

#[test]
fn carrega_projeto_minimo_sem_pastas_de_fontes_e_consultas() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("project.yaml"),
        "name: Vazio\nformat_version: 1\n",
    )
    .unwrap();

    let project = Project::load(dir.path()).unwrap();
    assert_eq!(project.name, "Vazio");
    assert_eq!(project.format_version, 1);
    assert!(project.sources.is_empty());
    assert!(project.queries.is_empty());
}

#[test]
fn ida_e_volta_com_todos_os_passos() {
    let dir = TempDir::new().unwrap();
    let project = common::all_steps_project();

    project.save(dir.path()).unwrap();
    let loaded = Project::load(dir.path()).unwrap();

    assert_eq!(loaded, project);
}

#[test]
fn ida_e_volta_das_variacoes_de_tipo_e_juncao() {
    let dir = TempDir::new().unwrap();
    let project = common::variations_project();

    project.save(dir.path()).unwrap();
    let loaded = Project::load(dir.path()).unwrap();

    assert_eq!(loaded, project);
}

#[test]
fn exemplo_da_secao_6_tem_saida_canonica() {
    let dir = TempDir::new().unwrap();
    common::example_project().save(dir.path()).unwrap();

    assert_eq!(
        fs::read_to_string(dir.path().join("project.yaml")).unwrap(),
        "name: Vendas\nformat_version: 1\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("sources/arquivo_vendas.yaml")).unwrap(),
        "id: arquivo_vendas\nkind: csv\npath: data/vendas.csv\n"
    );

    let query = fs::read_to_string(dir.path().join("queries/vendas.yaml")).unwrap();
    assert_eq!(query, common::CANONICAL_QUERY);
    assert!(query.ends_with('\n'));
    assert!(!query.contains('\r'));
}

#[test]
fn salvar_duas_vezes_produz_arquivos_identicos() {
    let dir = TempDir::new().unwrap();
    let project = common::all_steps_project();

    project.save(dir.path()).unwrap();
    let first = common::snapshot(dir.path());
    project.save(dir.path()).unwrap();
    let second = common::snapshot(dir.path());

    assert!(!first.is_empty());
    assert_eq!(first, second);
}

#[test]
fn remove_arquivos_obsoletos() {
    let dir = TempDir::new().unwrap();
    let mut project = common::all_steps_project();
    project.save(dir.path()).unwrap();

    assert!(dir.path().join("queries/vendas_completa.yaml").exists());
    assert!(dir.path().join("sources/produtos.yaml").exists());

    // Arquivos criados por fora e um que não é YAML.
    fs::write(
        dir.path().join("queries/antiga.yaml"),
        "id: antiga\nsource: arquivo_vendas\n",
    )
    .unwrap();
    fs::write(dir.path().join("sources/nota.txt"), "manter").unwrap();

    project.queries.retain(|query| query.id == "historico");
    project
        .sources
        .retain(|source| source.id == "arquivo_vendas");
    project.save(dir.path()).unwrap();

    assert!(dir.path().join("queries/historico.yaml").exists());
    assert!(!dir.path().join("queries/vendas_completa.yaml").exists());
    assert!(!dir.path().join("queries/antiga.yaml").exists());
    assert!(!dir.path().join("sources/produtos.yaml").exists());
    assert!(dir.path().join("sources/nota.txt").exists());
}

#[test]
fn consultas_ficam_em_ordem_de_dependencia() {
    let project = common::project(
        "Ordem",
        vec![common::source(
            "arquivo",
            SourceKind::Csv,
            "data/vendas.csv",
        )],
        vec![
            common::query("final", "resumo", vec![]),
            common::query("independente", "arquivo", vec![]),
            common::query("resumo", "vendas", vec![]),
            common::query("vendas", "arquivo", vec![]),
        ],
    );

    project.validate().unwrap();
    let order: Vec<&str> = project
        .query_order()
        .unwrap()
        .iter()
        .map(|query| query.id.as_str())
        .collect();

    assert_eq!(order, ["vendas", "resumo", "final", "independente"]);
}

#[test]
fn fontes_e_consultas_sao_ordenadas_por_id() {
    let dir = TempDir::new().unwrap();
    let project = common::project(
        "Ordenado",
        vec![
            common::source("z_fonte", SourceKind::Csv, "z.csv"),
            common::source("a_fonte", SourceKind::Csv, "a.csv"),
        ],
        vec![
            common::query("z_consulta", "a_fonte", vec![]),
            common::query("m_consulta", "a_fonte", vec![]),
            common::query("a_consulta", "a_fonte", vec![]),
        ],
    );

    project.save(dir.path()).unwrap();
    let loaded = Project::load(dir.path()).unwrap();

    let sources: Vec<&str> = loaded
        .sources
        .iter()
        .map(|source| source.id.as_str())
        .collect();
    let queries: Vec<&str> = loaded
        .queries
        .iter()
        .map(|query| query.id.as_str())
        .collect();
    assert_eq!(sources, ["a_fonte", "z_fonte"]);
    assert_eq!(queries, ["a_consulta", "m_consulta", "z_consulta"]);

    // A gravação também usa a ordem ordenada nos nomes dos arquivos.
    assert!(dir.path().join("sources/a_fonte.yaml").exists());
    assert!(dir.path().join("queries/a_consulta.yaml").exists());
}
