use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;

use crate::fixtures::{collect_structs, parse_program, resolve_path};
use crate::qualified_modules::generate_qualified_modules;
use crate::{AbiField, SharedMacroArgs, StructPath};

/// Organized qualified structs by module name.
/// Key: module name (e.g., "utxo" or "a_b" from "a::b::C")
/// Value: list of (struct name, fields)
pub(crate) type QualifiedModuleGroups = BTreeMap<String, Vec<(String, Vec<AbiField>)>>;

pub(crate) fn expand_noir_abi_shared_structs(args: SharedMacroArgs) -> syn::Result<TokenStream2> {
    let interface_module = &args.interface_module;
    let root_dir = resolve_path(&args.path)?;
    let program_paths = list_program_json_paths(&root_dir)?;

    let mut qualified_structs = BTreeMap::<StructPath, Vec<AbiField>>::new();
    for program_path in program_paths {
        let program = parse_program(&program_path)?;
        let mut structs = BTreeMap::new();
        for param in &program.abi.parameters {
            collect_structs(&param.field.ty, &mut structs);
        }

        for (path, fields) in structs {
            if !path.is_qualified() {
                continue;
            }

            match qualified_structs.get(&path) {
                Some(existing_fields) if existing_fields != &fields => {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        format!(
                            "qualified struct '{}::{}' has conflicting field definitions",
                            path.module, path.name
                        ),
                    ));
                }
                Some(_) => {}
                None => {
                    qualified_structs.insert(path, fields);
                }
            }
        }
    }

    let mut module_groups = QualifiedModuleGroups::new();
    for (path, fields) in qualified_structs {
        module_groups
            .entry(path.module)
            .or_default()
            .push((path.name, fields));
    }

    let modules = generate_qualified_modules(&module_groups, interface_module);
    Ok(quote! {
        #modules
    })
}

fn list_program_json_paths(root: &Path) -> syn::Result<Vec<PathBuf>> {
    let entries = fs::read_dir(root).map_err(|err| {
        syn::Error::new(
            Span::call_site(),
            format!(
                "failed to read fixtures directory {}: {}",
                root.display(),
                err
            ),
        )
    })?;

    let mut programs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let program_json = path.join("program.json");
        if program_json.is_file() {
            programs.push(program_json);
        }
    }

    programs.sort();
    Ok(programs)
}
