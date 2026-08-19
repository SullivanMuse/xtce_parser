use std::fs::File;
use std::io::Write;
use std::process::{Command, Output, Stdio};

use xsd_parser::models::meta::MetaTypeVariant;
use xsd_parser::{
    Config, Error,
    config::{GeneratorFlags, InterpreterFlags, OptimizerFlags, Resolver, Schema},
};
use xsd_parser::{
    MetaTypes, exec_generator, exec_interpreter, exec_optimizer, exec_parser, exec_render,
};

use quote::ToTokens;

fn main() -> Result<(), Error> {
    // This is almost the starting point defined in the main `[README.md]`.
    let mut config = Config::default()
        .with_schema(Schema::Url(
            "https://www.omg.org/spec/XTCE/20250214/SpaceSystem.xsd"
                .try_into()
                .unwrap(),
        ))
        .with_interpreter_flags(InterpreterFlags::all())
        .with_optimizer_flags(OptimizerFlags::all())
        .with_generator_flags(GeneratorFlags::all())
        .with_quick_xml();
    config.parser.resolver = vec![Resolver::Web];

    // Generate the code based on the configuration above.

    let schemas = exec_parser(config.parser)?;
    let meta_types = exec_interpreter(config.interpreter, &schemas)?;
    let meta_types = exec_optimizer(config.optimizer, meta_types)?;
    let meta_types = replace_variant_names(meta_types);
    let data_types = exec_generator(config.generator, &schemas, &meta_types)?;
    let module = exec_render(config.renderer, &data_types)?;
    let code = module.to_token_stream().to_string();

    let code = code.to_string();

    // Use a small helper to pretty-print the code (it uses `RUSTFMT`).
    // Actually, this is easier to use, if one has to compare the result of
    // 2 versions of `my-schema.xsd`.
    let code = rustfmt_pretty_print(code).unwrap();

    // Generate my_schema.rs, containing all structures and implementations defined from
    // `my-schema.xsd` and the configuration above.
    let mut file = File::create("src/my_schema.rs")?;
    file.write_all(code.to_string().as_bytes())?;

    Ok(())
}

// A small helper to call `rustfmt` when generating file(s).
// This may be useful to compare different versions of generated files.
pub fn rustfmt_pretty_print(code: String) -> Result<String, Error> {
    let mut child = Command::new("rustfmt")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();

    write!(stdin, "{code}")?;
    stdin.flush()?;
    drop(stdin);

    let Output {
        status,
        stdout,
        stderr,
    } = child.wait_with_output()?;

    let stdout = String::from_utf8_lossy(&stdout);
    let stderr = String::from_utf8_lossy(&stderr);

    if !status.success() {
        let code = status.code();
        match code {
            Some(code) => {
                if code != 0 {
                    panic!("The `rustfmt` command failed with return code {code}!\n{stderr}");
                }
            }
            None => {
                panic!("The `rustfmt` command failed!\n{stderr}")
            }
        }
    }

    Ok(stdout.into())
}

/// Define custom names for specific variants. Plus and minus characters are invalid identifiers in Rust.
fn replace_variant_names(mut types: MetaTypes) -> MetaTypes {
    for (_ident, ty) in types.items.iter_mut() {
        if let MetaTypeVariant::Enumeration(enum_meta) = &mut ty.variant {
            for variant in enum_meta.variants.iter_mut() {
                if let Some(value) = match variant.ident.name.as_str() {
                    "+" => Some("Plus"),
                    "-" => Some("Minus"),
                    "*" => Some("Times"),
                    "/" => Some("Divide"),
                    "%" => Some("Modulo"),
                    "^" => Some("Power"),
                    "y^x" => Some("ReversePower"),
                    "e^x" => Some("Exp"),
                    "1/x" => Some("Reciprocal"),
                    "x!" => Some("Factorial"),
                    "<<" => Some("ShiftLeft"),
                    ">>" => Some("ShiftRight"),
                    "&" => Some("BitAnd"),
                    "|" => Some("BitOr"),
                    "&&" => Some("And"),
                    "||" => Some("Or"),
                    "!" => Some("Not"),
                    ">" => Some("Gt"),
                    ">=" => Some("Ge"),
                    "<" => Some("Lt"),
                    "<=" => Some("Le"),
                    "==" => Some("Eq"),
                    "!=" => Some("Ne"),
                    "~" => Some("BitNot"),
                    _ => None,
                } {
                    variant.display_name = Some(value.to_string());
                }
            }
        }
    }

    types
}
