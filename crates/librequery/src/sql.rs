//! Utilitários de citação para o dialeto SQL usado pela biblioteca.
//!
//! Identificadores usam aspas duplas e literais de texto usam apóstrofos.
//! Em ambos os casos a aspa interna é escapada dobrando-a.

/// Cita um identificador com aspas duplas, dobrando aspas internas.
///
/// ```
/// assert_eq!(librequery::sql::quote_ident("vendas"), "\"vendas\"");
/// assert_eq!(librequery::sql::quote_ident("ve\"ndas"), "\"ve\"\"ndas\"");
/// ```
pub fn quote_ident(name: &str) -> String {
    let escaped = name.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

/// Cita um literal de texto com apóstrofos, dobrando apóstrofos internos.
///
/// ```
/// assert_eq!(librequery::sql::quote_literal("Solteiro"), "'Solteiro'");
/// assert_eq!(librequery::sql::quote_literal("O'Brien"), "'O''Brien'");
/// ```
pub fn quote_literal(value: &str) -> String {
    let escaped = value.replace('\'', "''");
    format!("'{escaped}'")
}
