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
    fn functions(&self) -> impl Iterator<Item = Function> {
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
    fn parameters(&self) -> impl Iterator<Item = Parameter> + 'a {
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

    fn write<T: Write>(&self, w: &mut T, with_body: bool) {
        write!(
            w,
            "\n\t/// {}\n\t#[allow(clippy::too_many_arguments)]\n\tfn {}(",
            self.description(),
            self.name()
        )
        .unwrap();

        write!(w, "&self").unwrap();
        let parameters = self.parameters();
        for parameter in parameters {
            if parameter.name != "apikey" && parameter.name != "function" {
                write!(w, ", ").unwrap();
                parameter.write(w, ParameterWriteMode::SuffixType)
            }
        }

        if !with_body {
            writeln!(w, ") -> JsonObject;").unwrap();
            return;
        }

        writeln!(w, ") -> JsonObject {{").unwrap();

        write!(
            w,
            "\t\tlet url = format!(\"https://www.alphavantage.co/query?"
        )
        .unwrap();

        let mut parameters = self.parameters().into_iter();

        write!(w, "{}{}", parameters.next().unwrap().name, "={}").unwrap();
        for parameter in parameters {
            write!(w, "&{}{}", parameter.name, "={}").unwrap();
        }

        write!(w, r#"""#).unwrap();
        for parameter in self.parameters() {
            write!(w, ", ").unwrap();

            match parameter.name.deref() {
                "apikey" => write!(w, "self.apikey").unwrap(),
                "function" => write!(w, r#""{}""#, self.raw_name()).unwrap(),
                _ => parameter.write(w, ParameterWriteMode::OnlyName),
            }
        }

        writeln!(w, ");\n\t\tself.client.get(&url)").unwrap();

        writeln!(w, "\t}}").unwrap();
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

/// Specify how this Parameter should be written to the writer
enum ParameterWriteMode {
    /// write the name attribute
    OnlyName,
    /// like Name but appends the type
    SuffixType,
}

impl Parameter {
    fn write<T: Write>(&self, w: &mut T, mode: ParameterWriteMode) {
        match mode {
            ParameterWriteMode::SuffixType => write!(w, "{}: &str", self.name),
            ParameterWriteMode::OnlyName => write!(w, "{}", self.name),
        }
        .unwrap();
    }
}

/// An Iterator which returns the Node following the current one
/// until there are no more node left.
fn following_nodes(node: Node) -> impl Iterator<Item = Node> {
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

    let parsed_sections: Vec<_> = main_content.find(Name("section")).map(Section).collect();

    for section in &parsed_sections {
        writeln!(&mut f, "pub trait {} {{", section.trait_name()).unwrap();

        for function in section.functions() {
            function.write(&mut f, false)
        }

        writeln!(&mut f, "}}\n").unwrap();

        writeln!(
            &mut f,
            "impl<'a, T> {} for AlphavantageClient<'a, T>\n\twhere T: RequestClient {{",
            section.trait_name()
        )
        .unwrap();

        for function in section.functions() {
            function.write(&mut f, true)
        }

        writeln!(&mut f, "}}\n").unwrap();
    }

    Command::new("rustfmt")
        .arg("--backup")
        .arg(&dest_path)
        .spawn()
        .unwrap();
}
