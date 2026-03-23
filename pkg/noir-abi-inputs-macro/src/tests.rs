use syn::{Type, parse_quote};

use crate::AbiField;
use crate::qualified_core::QualifiedModuleGroups;
use crate::qualified_modules::generate_qualified_modules;
use crate::struct_path::StructPath;

#[test]
fn qualified_type_path_for_circuit_modules_is_namespaced() {
    let path = StructPath::from_noir_path("signature::Foo");
    let tokens = path.type_path();
    let normalized = tokens.to_string().replace(' ', "");
    assert_eq!(normalized, "submodules::signature::Foo");
}

#[test]
fn generate_qualified_modules_wraps_namespace() {
    let interface_module: Type = parse_quote!(crate::circuits::proc_macro_interface);
    let mut groups = QualifiedModuleGroups::new();
    groups.insert(
        "signature".to_string(),
        vec![("foo".to_string(), Vec::<AbiField>::new())],
    );

    let tokens = generate_qualified_modules(&groups, &interface_module);
    let output = tokens.to_string();
    assert!(output.contains("pub mod submodules"));
    assert!(output.contains("pub mod signature"));
}
