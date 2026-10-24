//! Formatação de resultados para texto, usada em testes e depuração.

use datafusion::arrow::record_batch::RecordBatch;
use datafusion::arrow::util::display::array_value_to_string;

/// Formata os lotes como uma tabela de texto simples.
///
/// A primeira linha traz os nomes das colunas; cada linha seguinte é uma linha
/// dos lotes, com as células separadas por ` | `. Valores nulos aparecem como
/// texto vazio. Sem lotes, o resultado é uma string vazia.
pub fn format_batches(batches: &[RecordBatch]) -> String {
    let Some(first) = batches.first() else {
        return String::new();
    };

    let header = first
        .schema()
        .fields()
        .iter()
        .map(|field| field.name().clone())
        .collect::<Vec<_>>()
        .join(" | ");

    let mut lines = vec![header];
    for batch in batches {
        for row in 0..batch.num_rows() {
            let cells = batch
                .columns()
                .iter()
                .map(|column| array_value_to_string(column, row).unwrap_or_default())
                .collect::<Vec<_>>();
            lines.push(cells.join(" | "));
        }
    }
    lines.join("\n")
}
