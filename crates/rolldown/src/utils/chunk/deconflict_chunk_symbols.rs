use std::borrow::Cow;

use crate::{stages::link_stage::LinkStageOutput, utils::renamer::Renamer};
use rolldown_common::Chunk;
use rolldown_rstr::ToRstr;

#[tracing::instrument(level = "trace", skip_all)]
pub fn deconflict_chunk_symbols(chunk: &mut Chunk, link_output: &LinkStageOutput) {
  let mut renamer = Renamer::new(&link_output.symbols, link_output.module_table.modules.len());

  chunk
    .modules
    .iter()
    .copied()
    .filter_map(|id| link_output.module_table.modules[id].as_ecma())
    .flat_map(|m| m.scope.root_unresolved_references().keys().map(Cow::Borrowed))
    .for_each(|name| {
      // global names should be reserved
      renamer.reserve(Cow::Owned(name.to_rstr()));
    });

  // Though, those symbols in `imports_from_other_chunks` doesn't belong to this chunk, we still need to reserve them because we need to generate cross-chunk import
  // statements. Those `import {...} from './other-chunk.js'` will crate symbols in this chunk. These symbols also need to avoid conflicts.
  // So we add those in the deconflict process to generate conflict-less names in this chunk, and use these names to generate import statements.
  chunk.imports_from_other_chunks.iter().flat_map(|(_, items)| items.iter()).for_each(|item| {
    renamer.add_top_level_symbol(item.import_ref);
  });

  chunk
    .modules
    .iter()
    .copied()
    // Starts with entry module
    .rev()
    .filter_map(|id| link_output.module_table.modules[id].as_ecma())
    .for_each(|module| {
      module
        .stmt_infos
        .iter()
        .filter(|stmt_info| stmt_info.is_included)
        .flat_map(|stmt_info| stmt_info.declared_symbols.iter().copied())
        .for_each(|symbol_ref| {
          renamer.add_top_level_symbol(symbol_ref);
        });
    });

  // rename non-top-level names
  renamer.rename_non_top_level_symbol(&chunk.modules, &link_output.module_table.modules);

  chunk.canonical_names = renamer.into_canonical_names();
}
