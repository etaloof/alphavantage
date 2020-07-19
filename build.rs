use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Deref;
use std::path::Path;

use itertools::Itertools;
use regex::Regex;
use select::document::Document;
use select::node::Data;
use select::node::Node;
use select::predicate::*;
use std::process::Command;

use quote::*;
use proc_macro2::TokenStream;
use proc_macro2::Literal;

struct Section<'a>(Node<'a>);

impl<'a> Section<'a> {
    /// Generate a valid trait name from this section title
    fn trait_name(&self) -> String {
        let name = self.0.find(Name("h2")).nth(0).unwrap().text();

        // Remove anything in parentheses
        let invalid = Regex::new(r"\(.+?\)").unwrap();
        let name = invalid.replace(&name, "");

        let invalid = Regex::new("&").unwrap();
        invalid
            .replace(&name, "And")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("")
    }

    /// Generate a the functions from this section
    fn functions(&self) -> impl Iterator<Item=Function> {
        self.0.find(Name("h4")).map(Function)
    }
}

struct Function<'a>(Node<'a>);

impl<'a> Function<'a> {
    /// Extract the function name from this section
    fn name(&self) -> String {
        self.0
            .children()
            .nth(0)
            .unwrap()
            .text()
            .trim()
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("_")
    }

    fn raw_name(&self) -> String {
        let text = self.0.children().nth(0).unwrap().text();
        let name = text.trim();

        // Small hack: because of inconsistencies in the HTML documentation we need to replace these names
        let name = match name {
            "Quote Endpoint" => "GLOBAL_QUOTE",
            "Search Endpoint" => "SYMBOL_SEARCH",
            _ => name,
        };

        name.to_owned()
    }

    /// Extract the function description from this section
    fn description(&self) -> String {
        let node = self.0;

        following_nodes(node)
            .filter(|node| match node.data() {
                Data::Text(_) => false,
                Data::Element(_, _) if node.is(Name("br")) => false,
                _ => true,
            })
            .take_while(|node| !node.is(Name("h6")))
            .map(|node| node.text())
            .join(" ")
    }

    /// Generate the parameters for this function
    fn parameters(&self) -> impl Iterator<Item=Parameter> + 'a {
        let node = self.0;

        following_nodes(node)
            .filter(|node|
                match node.data() {
                    Data::Text(_) => false,
                    Data::Element(_, _) if node.is(Name("br")) => false,
                    _ => true,
                })
            .skip_while(|node| !node.is(Name("h6"))) // skip description
            .skip(1) // skip heading
            .take_while(|node| !node.is(Name("h6")))
            .batching(|it|
                if let Some(node) = it.next() {
                    Some((node, it.next()))
                } else { None })
            .filter_map(move |(parameter, extra)| {
                let extra = extra
                    .expect(&format!("Couldn't parse extra information for node {:?}", parameter));

                let name = match parameter.find(Name("code")).nth(0) {
                    Some(n) => n.text(),
                    _ => panic!("Couldn't parse parameter name (invalid value at {:?})!", parameter),
                };

                let necessity = match () {
                    _ if parameter.text().contains("Required") => ParameterNecessity::Required,
                    _ if parameter.text().contains("Optional") => ParameterNecessity::Optional(
                        match extra.find(Name("code")).nth(0) {
                            Some(n) => {
                                let default = n.text();
                                default.split("=").collect::<Vec<_>>()[1].to_owned()
                            }
                            _ => panic!("Couldn't parse parameter necessity default (invalid value at {:?})!", node),
                        }
                    ),
                    _ => panic!("Couldn't parse parameter necessity (invalid value at {:?})!", node),
                };

                // We only want responses in JSON so we skip the datatype parameter
                if name == "datatype" {
                    return None;
                }

                Some(Parameter { name, necessity })
            })
    }

    fn to_tokens_with_body(&self) -> TokenStream {
        use std::fmt::Write;
        let mut lit = format!("https://www.alphavantage.co/query?");
        let mut parameters = self.parameters().into_iter();
        write!(lit, "{}{}", parameters.next().unwrap().name, "={}").unwrap();
        for parameter in parameters {
            write!(lit, "&{}{}", parameter.name, "={}").unwrap();
        }
        let lit = Literal::string(&lit);

        let args = self.parameters()
            .into_iter()
            .map(|p| match p.name.deref() {
                "apikey" => quote!(self.apikey),
                "function" => Literal::string(&self.raw_name()).to_token_stream(),
                _ => format_ident!("{}", p.name).to_token_stream(),
            });

        let signature = self.to_tokens_head();
        quote!(
            #signature {
                let url = format!(#lit, #(#args),*);
                self.client.get(&url)
            }
        )
    }

    fn to_tokens_head(&self) -> TokenStream {
        let description = self.description();
        let name = format_ident!("{}", self.name());
        let parameters = self.parameters()
            .filter(|p| p.name != "apikey" && p.name != "function")
            .map(|x| format_ident!("{}", x.name))
            .map(|x| quote!(#x: &str));
        let parameters = std::iter::once(quote!(&self))
            .chain(parameters);
        quote!(
            #[doc = #description]
            fn #name(#(#parameters),*) -> JsonObject
        )
    }

    fn to_tokens(&self) -> TokenStream {
        let signature = self.to_tokens_head();
        quote!(
            #signature;
        )
    }
}

#[derive(Debug)]
enum ParameterNecessity {
    Required,
    Optional(String),
}

#[derive(Debug)]
struct Parameter {
    name: String,
    necessity: ParameterNecessity,
}

/// An Iterator which returns the Node following the current one
/// until there are no more node left.
fn following_nodes(node: Node) -> impl Iterator<Item=Node> {
    let mut node = node;
    std::iter::from_fn(move || {
        if let Some(next) = node.next() {
            node = next;
            Some(next)
        } else {
            None
        }
    })
}

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("gen.rs");
    let mut f = File::create(&dest_path).unwrap();

    const DOCUMENTATION: &str = include_str!("documentation.html");

    let document = Document::from(DOCUMENTATION);

    let main_content = document
        .find(Descendant(
            Attr("class", "container-fluid"),
            Name("article"),
        ))
        .nth(0)
        .unwrap();

    let parsed_sections: Vec<_> = main_content
        .find(Name("section"))
        .map(Section)
        .collect();

    for section in &parsed_sections {
        let trait_name = format_ident!("{}", section.trait_name());
        let functions = section.functions()
            .map(|function| function.to_tokens());

        let q = quote! {
            pub trait #trait_name {
                #(#functions) *
            }
        };
        writeln!(&mut f, "{}", q).unwrap();

        let functions = section.functions()
            .map(|function| function.to_tokens_with_body());
        let q = quote! {
            impl<'a, T> #trait_name for AlphavantageClient<'a, T>
                where T: RequestClient {
                    #(#functions) *
                }
        };
        writeln!(&mut f, "{}", q).unwrap();
    }

    Command::new("rustfmt")
        .arg("--backup")
        // definitely add fn_params_layout=vertical once rustfmt 2 is released
        .arg("--config")
        .arg("edition=2018,format_strings=true,wrap_comments=true,normalize_doc_attributes=true")
        .arg(&dest_path)
        .spawn()
        .unwrap();
}
