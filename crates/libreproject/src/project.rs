//! Implementação de [`Project`]: carga, gravação, validação, ordem de execução
//! e registro no motor.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use libreengine::Engine;
use librequery::{Query, Step};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::error::ProjectError;
use crate::format::{self, ProjectFile, QueryFile, SourceFile};

/// Versão de formato gravada por esta biblioteca.
pub const FORMAT_VERSION: u32 = 1;

/// Um projeto carregado em memória.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Project {
    /// Nome exibido do projeto.
    pub name: String,
    /// Versão do formato dos arquivos.
    pub format_version: u32,
    /// Fontes de dados, sem repetição de id.
    pub sources: Vec<SourceDef>,
    /// Consultas, sem repetição de id.
    pub queries: Vec<Query>,
}

/// Definição de uma fonte de dados do projeto.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDef {
    /// Identificador único entre fontes e consultas.
    pub id: String,
    /// Formato do arquivo de dados.
    pub kind: SourceKind,
    /// Caminho do arquivo, relativo à pasta do projeto.
    pub path: String,
}

/// Formato de uma fonte de dados.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    /// Arquivo CSV com cabeçalho.
    Csv,
    /// Arquivo Parquet.
    Parquet,
}

impl Project {
    /// Carrega o projeto da pasta `dir`.
    ///
    /// Os arquivos são lidos em ordem de nome, e as listas resultantes ficam
    /// ordenadas por id para que a carga seja determinística.
    pub fn load(dir: &Path) -> Result<Project, ProjectError> {
        let project_path = dir.join(format::PROJECT_FILE);
        let file: ProjectFile = read_yaml(&project_path)?;

        let mut sources = Vec::new();
        for path in yaml_files_in(&dir.join(format::SOURCES_DIR))? {
            let source: SourceFile = read_yaml(&path)?;
            sources.push(SourceDef::from(source));
        }
        sources.sort_by(|left, right| left.id.cmp(&right.id));

        let mut queries = Vec::new();
        for path in yaml_files_in(&dir.join(format::QUERIES_DIR))? {
            let query: QueryFile = read_yaml(&path)?;
            queries.push(Query::from(query));
        }
        queries.sort_by(|left, right| left.id.cmp(&right.id));

        migrate(Project {
            name: file.name,
            format_version: file.format_version,
            sources,
            queries,
        })
    }

    /// Grava o projeto em `dir` no formato canônico.
    ///
    /// O projeto é validado antes da gravação. Arquivos `.yaml` de `sources/`
    /// e `queries/` que não correspondam mais a um id do projeto são
    /// removidos; outros arquivos são preservados.
    pub fn save(&self, dir: &Path) -> Result<(), ProjectError> {
        self.validate()?;

        let sources_dir = dir.join(format::SOURCES_DIR);
        let queries_dir = dir.join(format::QUERIES_DIR);
        create_dir_all(dir)?;
        create_dir_all(&sources_dir)?;
        create_dir_all(&queries_dir)?;

        let project_file = ProjectFile {
            name: self.name.clone(),
            format_version: self.format_version,
        };
        write_yaml(&dir.join(format::PROJECT_FILE), &project_file)?;

        let mut sources: Vec<&SourceDef> = self.sources.iter().collect();
        sources.sort_by(|left, right| left.id.cmp(&right.id));
        for source in &sources {
            let path = sources_dir.join(format!("{}.yaml", source.id));
            write_yaml(&path, &SourceFile::from(*source))?;
        }
        remove_obsolete(
            &sources_dir,
            sources.iter().map(|source| source.id.as_str()),
        )?;

        let mut queries: Vec<&Query> = self.queries.iter().collect();
        queries.sort_by(|left, right| left.id.cmp(&right.id));
        for query in &queries {
            let path = queries_dir.join(format!("{}.yaml", query.id));
            write_yaml(&path, &QueryFile::from(*query))?;
        }
        remove_obsolete(&queries_dir, queries.iter().map(|query| query.id.as_str()))?;

        Ok(())
    }

    /// Confere ids, referências e ciclos do projeto.
    ///
    /// As regras são: ids únicos entre fontes e consultas (sem colisão entre os
    /// dois conjuntos); toda fonte de consulta e todo `other` de
    /// [`Step::Join`]/[`Step::Append`] precisa existir como fonte ou consulta;
    /// e as consultas não podem formar ciclo.
    pub fn validate(&self) -> Result<(), ProjectError> {
        let mut ids: HashSet<&str> = HashSet::new();

        for source in &self.sources {
            check_id(&source.id)?;
            if !ids.insert(source.id.as_str()) {
                return Err(ProjectError::DuplicateId {
                    id: source.id.clone(),
                });
            }
        }
        for query in &self.queries {
            check_id(&query.id)?;
            if !ids.insert(query.id.as_str()) {
                return Err(ProjectError::DuplicateId {
                    id: query.id.clone(),
                });
            }
        }

        for query in &self.queries {
            for reference in references(query) {
                if !ids.contains(reference) {
                    return Err(ProjectError::MissingReference {
                        query: query.id.clone(),
                        reference: reference.to_string(),
                    });
                }
            }
        }

        self.query_order()?;
        Ok(())
    }

    /// Devolve as consultas em ordem topológica, dependências primeiro.
    ///
    /// A ordem é determinística: a busca parte das consultas em ordem
    /// alfabética de id e cada dependência aparece antes de quem a usa. Um
    /// ciclo gera [`ProjectError::Cycle`].
    pub fn query_order(&self) -> Result<Vec<&Query>, ProjectError> {
        let mut by_id: BTreeMap<&str, &Query> = BTreeMap::new();
        for query in &self.queries {
            if by_id.insert(query.id.as_str(), query).is_some() {
                return Err(ProjectError::DuplicateId {
                    id: query.id.clone(),
                });
            }
        }

        let mut state: HashMap<&str, Visit> = HashMap::new();
        let mut order = Vec::with_capacity(by_id.len());
        for id in by_id.keys() {
            visit(id, &by_id, &mut state, &mut order)?;
        }
        Ok(order)
    }

    /// Registra no motor as fontes e depois as consultas do projeto.
    ///
    /// As consultas entram na ordem de [`Project::query_order`], de modo que
    /// `engine.run` funciona para qualquer consulta pelo seu id.
    pub async fn open_in(&self, engine: &Engine, dir: &Path) -> Result<(), ProjectError> {
        self.validate()?;

        for source in &self.sources {
            let path = dir.join(&source.path);
            let result = match source.kind {
                SourceKind::Csv => engine.register_csv(&source.id, &path).await,
                SourceKind::Parquet => engine.register_parquet(&source.id, &path).await,
            };
            result.map_err(|error| ProjectError::Engine {
                reference: source.id.clone(),
                source: error,
            })?;
        }

        for query in self.query_order()? {
            engine
                .register_query(query)
                .await
                .map_err(|error| ProjectError::Engine {
                    reference: query.id.clone(),
                    source: error,
                })?;
        }

        Ok(())
    }
}

/// Estado de uma consulta durante a busca em profundidade.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Visit {
    Visiting,
    Done,
}

/// Aplica as migrações de formato conhecidas.
///
/// Hoje só existe a versão 1; versões maiores são recusadas com mensagem clara
/// e versões menores (que ainda não têm caminho de migração) também. Este é o
/// ponto de entrada para migrações futuras.
fn migrate(project: Project) -> Result<Project, ProjectError> {
    let found = project.format_version;
    match found {
        version if version > FORMAT_VERSION => Err(ProjectError::UnsupportedVersion {
            found: version,
            supported: FORMAT_VERSION,
        }),
        FORMAT_VERSION => Ok(project),
        version => Err(ProjectError::UnsupportedVersion {
            found: version,
            supported: FORMAT_VERSION,
        }),
    }
}

/// Visita `id` e suas dependências, acrescentando-a depois delas em `order`.
fn visit<'a>(
    id: &'a str,
    by_id: &BTreeMap<&'a str, &'a Query>,
    state: &mut HashMap<&'a str, Visit>,
    order: &mut Vec<&'a Query>,
) -> Result<(), ProjectError> {
    match state.get(id) {
        Some(Visit::Done) => return Ok(()),
        Some(Visit::Visiting) => {
            return Err(ProjectError::Cycle {
                query: id.to_string(),
            });
        }
        None => {}
    }

    let Some(&query) = by_id.get(id) else {
        return Ok(());
    };

    state.insert(id, Visit::Visiting);
    for reference in references(query) {
        if by_id.contains_key(reference) {
            visit(reference, by_id, state, order)?;
        }
    }
    state.insert(id, Visit::Done);
    order.push(query);
    Ok(())
}

/// Fontes citadas por uma consulta: a origem e os `other` de junção e união.
fn references(query: &Query) -> Vec<&str> {
    let mut references = vec![query.source.as_str()];
    for step in &query.steps {
        match step {
            Step::Join { other, .. } | Step::Append { other } => {
                references.push(other.as_str());
            }
            _ => {}
        }
    }
    references
}

/// Recusa identificadores que não podem virar nome de arquivo.
fn check_id(id: &str) -> Result<(), ProjectError> {
    if id.is_empty() || id == "." || id == ".." || id.contains('/') || id.contains('\\') {
        return Err(ProjectError::InvalidId { id: id.to_string() });
    }
    Ok(())
}

/// Lê e converte um arquivo YAML.
fn read_yaml<T: DeserializeOwned>(path: &Path) -> Result<T, ProjectError> {
    let text = fs::read_to_string(path).map_err(|source| ProjectError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_yaml_ng::from_str(&text).map_err(|source| ProjectError::Yaml {
        path: path.to_path_buf(),
        source,
    })
}

/// Grava um valor em YAML canônico, com quebra de linha final `\n`.
fn write_yaml<T: Serialize>(path: &Path, value: &T) -> Result<(), ProjectError> {
    let mut text = serde_yaml_ng::to_string(value).map_err(|source| ProjectError::Yaml {
        path: path.to_path_buf(),
        source,
    })?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    let text = text.replace("\r\n", "\n");
    fs::write(path, text).map_err(|source| ProjectError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Lista os arquivos `.yaml` da pasta, em ordem de nome; pasta ausente é vazia.
fn yaml_files_in(dir: &Path) -> Result<Vec<PathBuf>, ProjectError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(ProjectError::Io {
                path: dir.to_path_buf(),
                source,
            });
        }
    };

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| ProjectError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "yaml")
        {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Cria a pasta e todas as intermediárias.
fn create_dir_all(path: &Path) -> Result<(), ProjectError> {
    fs::create_dir_all(path).map_err(|source| ProjectError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Remove os `.yaml` da pasta cujo nome não está em `keep`.
fn remove_obsolete<'a>(
    dir: &Path,
    keep: impl Iterator<Item = &'a str>,
) -> Result<(), ProjectError> {
    let keep: HashSet<&str> = keep.collect();
    for path in yaml_files_in(dir)? {
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        if !keep.contains(stem) {
            fs::remove_file(&path).map_err(|source| ProjectError::Io {
                path: path.clone(),
                source,
            })?;
        }
    }
    Ok(())
}
