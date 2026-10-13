//! `cli.parse[T]`, the stage-0 stand-in for the derive-based parser of
//! `std.x.cli` (contract in compiler/std/cli.sisu).

use crate::value::Value;
use crate::{Interp, Unwind, ice};
use sisu_sir as sir;

enum CliError {
    UnknownOption(String),
    MissingValue(String),
    InvalidValue(String, String),
    MissingArgument(String),
    UnexpectedArgument(String),
}

fn option_name(field: &str) -> String {
    format!("--{}", field.replace('_', "-"))
}

fn convert(ty: &sir::Type, text: &str) -> Option<Value> {
    match ty {
        sir::Type::Int => text.parse().ok().map(Value::Int),
        sir::Type::Float => text.parse().ok().map(Value::Float),
        sir::Type::Str => Some(Value::str(text)),
        sir::Type::Bool => match text {
            "true" | "True" => Some(Value::Bool(true)),
            "false" | "False" => Some(Value::Bool(false)),
            _ => None,
        },
        sir::Type::Named(n, _) if n == "String" => Some(Value::String(text.to_string())),
        sir::Type::Named(n, _) if n == "Path" => Some(Value::Path(text.into())),
        _ => None,
    }
}

impl Interp<'_, '_> {
    pub(crate) fn cli_parse(&mut self, ty: Option<&sir::Type>, args: &Value) -> Result<Value, Unwind> {
        let Some(sir::Type::Named(name, _)) = ty else {
            return Err(ice("cli.parse without a type argument"));
        };
        let Some(sir::TypeDecl::Struct { fields, .. }) = self.types.get(name.as_str()).copied() else {
            return Err(ice(format!("cli.parse of `{name}`, which is not a struct")));
        };
        let argv: Vec<String> = match self.target(args)? {
            Value::List(xs) => {
                xs.iter().map(|x| self.text(x).map(str::to_string)).collect::<Result<_, _>>()?
            }
            other => return Err(ice(format!("cli.parse of {other:?}"))),
        };
        match self.cli_fields(name, fields, &argv)? {
            Ok(v) => Ok(Value::ok(v)),
            Err(e) => {
                let (variant, fields) = match e {
                    CliError::UnknownOption(n) => ("UnknownOption", vec![Value::String(n)]),
                    CliError::MissingValue(n) => ("MissingValue", vec![Value::String(n)]),
                    CliError::InvalidValue(n, v) => {
                        ("InvalidValue", vec![Value::String(n), Value::String(v)])
                    }
                    CliError::MissingArgument(n) => ("MissingArgument", vec![Value::String(n)]),
                    CliError::UnexpectedArgument(v) => ("UnexpectedArgument", vec![Value::String(v)]),
                };
                let i = self
                    .variants
                    .get(&("cli.Error", variant))
                    .copied()
                    .ok_or_else(|| ice("cli.Error is not declared"))?;
                Ok(Value::err(Value::variant("cli.Error", i, fields)))
            }
        }
    }

    fn cli_fields(
        &mut self,
        name: &str,
        fields: &[sir::FieldDecl],
        argv: &[String],
    ) -> Result<Result<Value, CliError>, Unwind> {
        let positional = fields.iter().position(|f| matches!(&f.ty, sir::Type::Named(n, _) if n == "list"));
        let mut vals: Vec<Option<Value>> = vec![None; fields.len()];
        let mut items: Vec<String> = Vec::new();
        let mut i = 0;
        let mut options_done = false;
        while i < argv.len() {
            let a = &argv[i];
            i += 1;
            if !options_done && a == "--" {
                options_done = true;
                continue;
            }
            let Some(body) = a.strip_prefix("--").filter(|b| !options_done && !b.is_empty()) else {
                if positional.is_none() {
                    return Ok(Err(CliError::UnexpectedArgument(a.clone())));
                }
                items.push(a.clone());
                continue;
            };
            let (written, inline) = match body.split_once('=') {
                Some((n, v)) => (n, Some(v)),
                None => (body, None),
            };
            let field = written.replace('-', "_");
            let Some(fi) = fields.iter().position(|f| f.name == field).filter(|fi| Some(*fi) != positional)
            else {
                return Ok(Err(CliError::UnknownOption(format!("--{written}"))));
            };
            let f = &fields[fi];
            if f.ty == sir::Type::Bool && inline.is_none() {
                vals[fi] = Some(Value::Bool(true));
                continue;
            }
            let value = match inline {
                Some(v) => v.to_string(),
                None => match argv.get(i) {
                    Some(v) => {
                        i += 1;
                        v.clone()
                    }
                    None => return Ok(Err(CliError::MissingValue(format!("--{written}")))),
                },
            };
            match convert(&f.ty, &value) {
                Some(v) => vals[fi] = Some(v),
                None => return Ok(Err(CliError::InvalidValue(format!("--{written}"), value))),
            }
        }
        for (fi, f) in fields.iter().enumerate() {
            if Some(fi) == positional {
                let elem = match &f.ty {
                    sir::Type::Named(_, args) => args.first().cloned().unwrap_or(sir::Type::Str),
                    _ => sir::Type::Str,
                };
                let mut list = Vec::new();
                for it in &items {
                    match convert(&elem, it) {
                        Some(v) => list.push(v),
                        None => return Ok(Err(CliError::InvalidValue(f.name.clone(), it.clone()))),
                    }
                }
                vals[fi] = Some(Value::List(list));
                continue;
            }
            if vals[fi].is_some() {
                continue;
            }
            vals[fi] = Some(match &f.default {
                Some(d) => self.call_by_name(d, Vec::new())?,
                None if f.ty == sir::Type::Bool => Value::Bool(false),
                None => return Ok(Err(CliError::MissingArgument(option_name(&f.name)))),
            });
        }
        Ok(Ok(Value::Struct(Value::adt(
            name,
            0,
            vals.into_iter().map(|v| v.unwrap_or(Value::Unit)).collect(),
        ))))
    }
}
