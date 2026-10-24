//! Erros de leitura, gravação, validação e execução de projetos.

use std::fmt;
use std::io;
use std::path::PathBuf;

use libreengine::EngineError;

/// Erro ao carregar, validar, salvar ou abrir um projeto no motor.
#[derive(Debug)]
pub enum ProjectError {
    /// Falha de entrada/saída, com o caminho que a provocou.
    Io { path: PathBuf, source: io::Error },
    /// Arquivo YAML ausente de campos, malformado ou com operação desconhecida.
    Yaml {
        path: PathBuf,
        source: serde_yaml_ng::Error,
    },
    /// O `format_version` do projeto não é suportado por esta biblioteca.
    UnsupportedVersion { found: u32, supported: u32 },
    /// O mesmo identificador aparece mais de uma vez.
    DuplicateId { id: String },
    /// O identificador não serve como nome de arquivo.
    InvalidId { id: String },
    /// Uma consulta cita uma fonte ou consulta inexistente.
    MissingReference { query: String, reference: String },
    /// As consultas dependem umas das outras em ciclo.
    Cycle { query: String },
    /// O motor recusou registrar uma fonte ou uma consulta.
    Engine {
        reference: String,
        source: EngineError,
    },
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "falha de entrada/saída em {}: {source}", path.display())
            }
            Self::Yaml { path, source } => {
                write!(f, "YAML inválido em {}: {source}", path.display())
            }
            Self::UnsupportedVersion { found, supported } => write!(
                f,
                "versão de formato {found} não suportada; esta versão entende a versão {supported}"
            ),
            Self::DuplicateId { id } => write!(f, "id duplicado: \"{id}\""),
            Self::InvalidId { id } => write!(f, "id inválido para nome de arquivo: \"{id}\""),
            Self::MissingReference { query, reference } => write!(
                f,
                "a consulta \"{query}\" referencia \"{reference}\", que não existe como fonte nem como consulta"
            ),
            Self::Cycle { query } => {
                write!(f, "as consultas formam um ciclo envolvendo \"{query}\"")
            }
            Self::Engine { reference, source } => {
                write!(f, "falha no motor ao registrar \"{reference}\": {source}")
            }
        }
    }
}

impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Yaml { source, .. } => Some(source),
            Self::Engine { source, .. } => Some(source),
            Self::UnsupportedVersion { .. }
            | Self::DuplicateId { .. }
            | Self::InvalidId { .. }
            | Self::MissingReference { .. }
            | Self::Cycle { .. } => None,
        }
    }
}
