use std::collections::HashMap;

use proc_macro2::LineColumn;
use sha2::{Digest, Sha256};
use syn::{Attribute, File, ItemFn, spanned::Spanned as _, visit::Visit};

pub(crate) fn test_body_sha256s(source: &str) -> HashMap<String, String> {
    let normalized = source.replace("\r\n", "\n");
    let syntax: File = syn::parse_file(&normalized).expect("parse current fixture source");
    let mut visitor = TestBodyVisitor::new(&normalized);
    visitor.visit_file(&syntax);
    visitor.matches
}

struct TestBodyVisitor<'source> {
    source: &'source str,
    offsets: Vec<usize>,
    matches: HashMap<String, String>,
}

impl<'source> TestBodyVisitor<'source> {
    fn new(source: &'source str) -> Self {
        let mut offsets = vec![0];
        for (index, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                offsets.push(index + 1);
            }
        }
        Self {
            source,
            offsets,
            matches: HashMap::new(),
        }
    }

    fn offset(&self, position: LineColumn) -> usize {
        self.offsets[position.line - 1] + position.column
    }

    fn body(&self, item: &ItemFn) -> &[u8] {
        let span = item.block.span();
        &self.source.as_bytes()[self.offset(span.start())..self.offset(span.end())]
    }
}

impl<'ast> Visit<'ast> for TestBodyVisitor<'_> {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        if is_test(&item.attrs) {
            let name = item.sig.ident.to_string();
            let digest = sha256_hex(self.body(item));
            assert!(
                self.matches.insert(name.clone(), digest).is_none(),
                "duplicate current fixture test name: {name}"
            );
        }
        syn::visit::visit_item_fn(self, item);
    }
}

fn is_test(attributes: &[Attribute]) -> bool {
    attributes
        .iter()
        .any(|attribute| attribute.path().is_ident("test"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("write SHA-256 to String");
    }
    encoded
}
