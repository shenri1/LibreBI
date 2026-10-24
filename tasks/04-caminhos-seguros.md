# Tarefa 04 — LibreProject: caminhos de fontes seguros

Fonte de requisitos: apenas `docs/` (leia `AGENTS.md`; depois `docs/00-especificacao-mvp.md` seção 7.4 sobre confiança em projetos).
Não consulte nada fora deste diretório de trabalho.

## Problema
O campo `path` de uma fonte (`sources/<id>.yaml`) é relativo à pasta do projeto, mas hoje nada impede que aponte para fora dela
(`../../x.csv`) ou para um caminho absoluto. Um projeto recebido de terceiros poderia fazer o motor ler arquivos arbitrários do
computador do usuário.

## O que fazer (em `crates/libreproject`)
1. Em `Project::validate` (e, portanto, em `load`, `save` e `open_in`), rejeite com um erro novo e claro
   (`ProjectError::UnsafePath`, mensagem em português com o id da fonte e o caminho) qualquer `path` que: seja absoluto
   (inclusive com letra de unidade ou `\` no Windows), contenha componentes `..`, esteja vazio, ou contenha o byte nulo.
2. Em `open_in`, além de validar o texto, resolva o caminho junto à pasta do projeto e confirme, depois de canonicalizar, que o
   arquivo está dentro da pasta canônica do projeto. Isso cobre links simbólicos que saem da pasta. Se o arquivo não existir,
   mantenha o erro atual de arquivo não encontrado.
3. Testes: caminho absoluto, `..` no meio e no começo, vazio, e um link simbólico que escapa (pule esse teste com
   `#[cfg(unix)]`, ou com tratamento adequado no Windows, se a criação do link exigir privilégio). Os testes existentes
   e o exemplo `examples/vendas` devem continuar passando.
4. Rode `cargo fmt`, `cargo test --workspace` e `cargo build`. Atualize o README do crate com uma frase sobre a regra.

## Restrições
Mudança pequena e localizada, sem novas dependências. Responda no final com 3 linhas: arquivos alterados, testes novos, resultado do cargo test.
