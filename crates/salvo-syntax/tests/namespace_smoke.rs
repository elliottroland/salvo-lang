//! [namespace] Smoke test: `namespace X { … }` flattens to dotted names.

use salvo_syntax::ast::Item;
use salvo_syntax::parse_module;

fn item_names(src: &str) -> Vec<String> {
    let (module, diags) = parse_module(src);
    let errs: Vec<_> = diags.iter().filter(|d| d.is_error()).collect();
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    module
        .items
        .iter()
        .map(|i| match i {
            Item::Struct(s) => format!("struct {}", s.name.name),
            Item::Type(t) => format!("type {}{}", t.name.name, if t.exported { " [export]" } else { "" }),
            Item::Qualifier(q) => format!("qualifier {}{}", q.name.name, if q.exported { " [export]" } else { "" }),
            Item::Fn(f) => format!("fn {}", f.name.name),
            Item::Effect(e) => format!("effect {}", e.name.name),
            Item::Namespace(_) => "NAMESPACE (unflattened!)".to_string(),
            _ => "other".to_string(),
        })
        .collect()
}

#[test]
fn flat_namespace_members_get_dotted_names() {
    let src = "\
namespace AttributeValue {
    qualifier S of Str
    qualifier N of Str
    qualifier M of Str
}
";
    let names = item_names(src);
    assert_eq!(
        names,
        vec![
            "qualifier AttributeValue.S",
            "qualifier AttributeValue.N",
            "qualifier AttributeValue.M",
        ]
    );
}

#[test]
fn export_namespace_distributes() {
    let src = "\
export namespace Foo {
    type Bar = Bar.A | Bar.B
    struct Bar.A {}
    struct Bar.B {}
}
";
    let names = item_names(src);
    assert!(names.contains(&"type Foo.Bar [export]".to_string()), "{names:?}");
    assert!(names.iter().any(|n| n == "struct Foo.Bar.A"), "{names:?}");
}

#[test]
fn nested_namespaces_compose() {
    let src = "\
namespace Outer {
    namespace Inner {
        struct Thing {}
        fn helper() -> None {}
    }
    struct Top {}
}
";
    let names = item_names(src);
    assert!(names.iter().any(|n| n == "struct Outer.Inner.Thing"), "{names:?}");
    assert!(names.iter().any(|n| n == "fn Outer.Inner.helper"), "{names:?}");
    assert!(names.iter().any(|n| n == "struct Outer.Top"), "{names:?}");
}

#[test]
fn terminal_namespace_takes_rest_of_file() {
    let src = "\
struct Before {}

namespace Rest ...

struct After {}
fn tail() -> None {}
";
    let names = item_names(src);
    assert!(names.iter().any(|n| n == "struct Before"), "{names:?}");
    assert!(names.iter().any(|n| n == "struct Rest.After"), "{names:?}");
    assert!(names.iter().any(|n| n == "fn Rest.tail"), "{names:?}");
}
