# Regras do projeto

Estas regras valem para qualquer agente ou pessoa que escreva código neste repositório.

## Fonte única de informação
- A **única** fonte de requisitos e de comportamento esperado é a pasta local `docs/` (não versionada; está no `.gitignore`).
  Comece por `docs/00-especificacao-mvp.md` e `docs/INDEX.md`.
- Não consulte, abra ou copie material de análise, código-fonte ou binários de produtos de terceiros, nem qualquer coisa
  fora deste diretório de trabalho que descreva ou contenha o comportamento interno de outro software.
- Não use conhecimento sobre o código-fonte interno de produtos de terceiros. Se algo não estiver em `docs/`, trate como
  **decisão de design própria**: escolha a solução mais simples e registre o que foi decidido (por exemplo em `tasks/` ou em um ADR).
- Se `docs/` for ambígua ou omissa, não preencha a lacuna com lembrança de outro produto: pergunte ou registre como pendência.

## Como trabalhar
- Código, nomes e estruturas são **originais**. Não reproduza a decomposição interna de nenhum produto existente.
- Mudanças pequenas, com critério de pronto. Um passo por vez, na ordem de `docs/produto/02-roadmap.md`.
- Dependências mínimas; adicione uma nova só quando o passo atual exigir.
- Sem telemetria. Comentários e documentação em português; identificadores em inglês.
- Antes de concluir um passo: `cargo check` e `cargo test` sem erros.
