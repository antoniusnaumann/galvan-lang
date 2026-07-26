//! `textDocument/typeDefinition` — jump to the declaration of a symbol's
//! *type*: from a variable, parameter or field to its type declaration,
//! and from an enum case to the enum.

use std::path::Path;

use galvan_hir::DefinitionKind;
use tower_lsp::lsp_types::{Location, Position};

use crate::analysis::{self, receiver_type_name};
use crate::document::Document;
use crate::features::Locations;
use crate::workspace::Crate;

pub fn type_definition(
    current: &Document,
    krate: &Crate,
    file: Option<&Path>,
    position: Position,
) -> Option<Location> {
    let offset = current.line_index.offset(&current.text, position)?;
    let analysis = krate.analyze()?;
    let file = file?;
    let (_, definition) = analysis::symbol_at(&analysis.index, file, offset)?;

    let type_name = match &definition.kind {
        // On a type itself, its own declaration is the type definition.
        DefinitionKind::Type => definition.name.clone(),
        DefinitionKind::EnumVariant { owner } => owner.to_string(),
        // Functions have no single type to jump to.
        DefinitionKind::Function { .. } => return None,
        _ => receiver_type_name(definition.ty()?)?.to_string(),
    };

    let (_, type_definition) = analysis
        .index
        .definitions()
        .find(|(_, definition)| {
            matches!(definition.kind, DefinitionKind::Type) && definition.name == type_name
        })?;
    let site = analysis::definition_site(type_definition)?;
    Locations::new().location(site.source, site.target.range.0, site.target.range.1)
}
