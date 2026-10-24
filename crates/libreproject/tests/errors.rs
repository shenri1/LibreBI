mod common;

use std::fs;
use std::path::Path;

use libreengine::Engine;
use libreproject::{Project, ProjectError, SourceKind};
use librequery::{JoinKind, Step};
use tempfile::TempDir;

/// Pasta com as subpastas vazias e um `project.yaml` válido.
fn skeletons(format_version: u32) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("sources")).unwrap();
    fs::create_dir_all(dir.path().join("queries")).unwrap();
    write_project(dir.path(), format_version);
    dir
}

/// Grava apenas o `project.yaml`.
fn write_project(dir: &Path, format_version: u32) {
    fs::write(
        dir.join("project.yaml"),
        format!("name: Teste\nformat_version: {format_version}\n"),
    )
    .unwrap();
}

#[test]
fn id_duplicado_entre_fontes() {
    let project = common::project(
        "Duplicado",
        vec![
            common::source("fonte", SourceKind::Csv, "a.csv"),
            common::source("fonte", SourceKind::Parquet, "b.parquet"),
        ],
        vec![],
    );

    let error = project.validate().unwrap_err();
    assert!(matches!(&error, ProjectError::DuplicateId { id } if id == "fonte"));
    assert_eq!(error.to_string(), "id duplicado: \"fonte\"");
}

#[test]
fn id_duplicado_entre_fonte_e_consulta() {
    let project = common::project(
        "Colisão",
        vec![common::source("base", SourceKind::Csv, "base.csv")],
        vec![common::query("base", "base", vec![])],
    );

    let error = project.validate().unwrap_err();
    assert!(matches!(&error, ProjectError::DuplicateId { id } if id == "base"));
}

#[test]
fn referencia_de_fonte_inexistente() {
    let project = common::project(
        "Referência",
        vec![],
        vec![common::query("q", "nao_existe", vec![])],
    );

    let error = project.validate().unwrap_err();
    assert!(matches!(
        &error,
        ProjectError::MissingReference { query, reference }
            if query == "q" && reference == "nao_existe"
    ));
    assert_eq!(
        error.to_string(),
        "a consulta \"q\" referencia \"nao_existe\", que não existe como fonte nem como consulta"
    );
}

#[test]
fn referencia_inexistente_em_join() {
    let project = common::project(
        "Junção",
        vec![common::source("base", SourceKind::Csv, "base.csv")],
        vec![common::query(
            "q",
            "base",
            vec![Step::Join {
                kind: JoinKind::Inner,
                other: "fantasma".to_string(),
                left_key: "a".to_string(),
                right_key: "a".to_string(),
            }],
        )],
    );

    let error = project.validate().unwrap_err();
    assert!(matches!(
        &error,
        ProjectError::MissingReference { query, reference }
            if query == "q" && reference == "fantasma"
    ));
}

#[test]
fn referencia_inexistente_em_append() {
    let project = common::project(
        "União",
        vec![common::source("base", SourceKind::Csv, "base.csv")],
        vec![common::query(
            "q",
            "base",
            vec![Step::Append {
                other: "fantasma".to_string(),
            }],
        )],
    );

    let error = project.validate().unwrap_err();
    assert!(matches!(
        &error,
        ProjectError::MissingReference { query, reference }
            if query == "q" && reference == "fantasma"
    ));
}

#[test]
fn ciclo_entre_consultas() {
    let project = common::project(
        "Ciclo",
        vec![],
        vec![
            common::query("a", "b", vec![]),
            common::query("b", "a", vec![]),
        ],
    );

    let error = project.validate().unwrap_err();
    assert!(matches!(&error, ProjectError::Cycle { query } if query == "a"));
    assert_eq!(
        error.to_string(),
        "as consultas formam um ciclo envolvendo \"a\""
    );
}

#[test]
fn consulta_que_depende_de_si_mesma() {
    let project = common::project("Autociclo", vec![], vec![common::query("a", "a", vec![])]);

    let error = project.query_order().unwrap_err();
    assert!(matches!(&error, ProjectError::Cycle { query } if query == "a"));
}

#[test]
fn id_invalido_para_nome_de_arquivo() {
    let project = common::project(
        "Inválido",
        vec![common::source("../fora", SourceKind::Csv, "fora.csv")],
        vec![],
    );

    let error = project.validate().unwrap_err();
    assert!(matches!(&error, ProjectError::InvalidId { id } if id == "../fora"));
    assert_eq!(
        error.to_string(),
        "id inválido para nome de arquivo: \"../fora\""
    );
}

#[test]
fn project_yaml_invalido() {
    let dir = skeletons(1);
    fs::write(dir.path().join("project.yaml"), "name: [\n").unwrap();

    let error = Project::load(dir.path()).unwrap_err();
    assert!(matches!(&error, ProjectError::Yaml { path, .. } if path.ends_with("project.yaml")));
    assert!(error.to_string().starts_with("YAML inválido em "));
}

#[test]
fn arquivo_de_fonte_invalido() {
    let dir = skeletons(1);
    fs::write(dir.path().join("sources/quebrada.yaml"), "id: [\n").unwrap();

    let error = Project::load(dir.path()).unwrap_err();
    assert!(matches!(&error, ProjectError::Yaml { path, .. } if path.ends_with("quebrada.yaml")));
}

#[test]
fn op_desconhecido() {
    let dir = skeletons(1);
    fs::write(
        dir.path().join("sources/base.yaml"),
        "id: base\nkind: csv\npath: base.csv\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("queries/q.yaml"),
        "id: q\nsource: base\nsteps:\n- op: pivot\n  columns:\n  - a\n",
    )
    .unwrap();

    let error = Project::load(dir.path()).unwrap_err();
    assert!(matches!(&error, ProjectError::Yaml { path, .. } if path.ends_with("q.yaml")));
    assert!(error.to_string().contains("pivot"));
}

#[test]
fn versao_de_formato_maior_que_a_suportada() {
    let dir = TempDir::new().unwrap();
    write_project(dir.path(), 2);

    let error = Project::load(dir.path()).unwrap_err();
    assert!(matches!(
        &error,
        ProjectError::UnsupportedVersion {
            found: 2,
            supported: 1
        }
    ));
    assert_eq!(
        error.to_string(),
        "versão de formato 2 não suportada; esta versão entende a versão 1"
    );
}

#[test]
fn versao_de_formato_anterior_a_suportada() {
    let dir = TempDir::new().unwrap();
    write_project(dir.path(), 0);

    let error = Project::load(dir.path()).unwrap_err();
    assert!(matches!(
        &error,
        ProjectError::UnsupportedVersion {
            found: 0,
            supported: 1
        }
    ));
}

#[test]
fn pasta_inexistente() {
    let dir = TempDir::new().unwrap();

    let error = Project::load(&dir.path().join("nao_existe")).unwrap_err();
    assert!(matches!(&error, ProjectError::Io { path, .. } if path.ends_with("project.yaml")));
    assert!(error.to_string().starts_with("falha de entrada/saída em "));
}

#[tokio::test]
async fn open_in_devolve_erro_do_motor_quando_falta_arquivo() {
    let dir = TempDir::new().unwrap();
    let project = common::example_project();
    project.save(dir.path()).unwrap();

    let engine = Engine::new();
    let error = project.open_in(&engine, dir.path()).await.unwrap_err();

    assert!(matches!(
        &error,
        ProjectError::Engine { reference, .. } if reference == "arquivo_vendas"
    ));
    assert!(
        error
            .to_string()
            .starts_with("falha no motor ao registrar \"arquivo_vendas\"")
    );
}
