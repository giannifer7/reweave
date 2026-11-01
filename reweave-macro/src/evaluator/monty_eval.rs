use monty::MontyRun;
use monty_types::{CompileOptions, MontyObject};
use std::collections::{HashMap, HashSet};
pub struct MontyEvaluator;

impl Default for MontyEvaluator {
    fn default() -> Self {
        Self::new()
    }
}
impl MontyEvaluator {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        code: &str,
        params: &[String],
        args: &[String],
        store: &HashMap<String, String>,
        name: Option<&str>,
    ) -> Result<String, String> {
        let macro_name = name.unwrap_or("pydef");

        // Inject store entries as additional parameters that come before the
        // declared params. Declared params shadow any store key with the same name.
        let param_set: HashSet<&str> = params.iter().map(String::as_str).collect();
        let mut all_params: Vec<String> = store
            .keys()
            .filter(|k| !param_set.contains(k.as_str()))
            .cloned()
            .collect();
        all_params.extend_from_slice(params);

        let mut all_args: Vec<MontyObject> = store
            .iter()
            .filter(|(k, _)| !param_set.contains(k.as_str()))
            .map(|(_, v)| MontyObject::string(v.clone()))
            .collect();
        all_args.extend(args.iter().map(|s| MontyObject::string(s.clone())));

        let mut runner = MontyRun::new(
            code.to_owned(),
            &format!("{macro_name}.py"),
            all_params,
            CompileOptions::default(),
        )
        .map_err(|e| format!("pydef '{macro_name}': compile error: {e:?}"))?;

        let result = runner
            .run_no_limits(all_args)
            .map_err(|e| format!("pydef '{macro_name}': runtime error: {e:?}"))?;

        Ok(monty_object_to_string(result))
    }
}

/// Converts a [`MontyObject`] result to the string a `%pydef` call expands
/// to: `None` and `False` are empty, `True` is `"true"`, numbers and strings
/// render as their value, and a list concatenates its items (recursively).
/// Any other type (dict, tuple, exception, ...) falls back to its Python
/// `repr()`, so it is still visible rather than silently dropped.
pub fn monty_object_to_string(obj: MontyObject) -> String {
    monty_ref_to_string(obj.as_ref())
}

fn monty_ref_to_string(obj: monty_types::ObjectRef<'_>) -> String {
    match obj.type_name() {
        "NoneType" => String::new(),
        "bool" => {
            if obj.as_bool().unwrap_or(false) {
                "true".into()
            } else {
                String::new()
            }
        }
        // `as_int` is `None` only for an `int` too large for `i64` (monty's
        // `BigInt`); its `repr()` is the same digit string Python's `str()`
        // gives, so the fallback stays correct for an arbitrarily large int.
        "int" => obj.as_int().map_or_else(|| obj.py_repr(), |i| i.to_string()),
        // `as_float` is always `Some` once `type_name()` is `"float"` (the
        // only `None` cases are non-numeric types, already routed to other
        // arms), so unlike `"int"` there is no large-value fallback to take.
        "float" => obj
            .as_float()
            .expect("type_name() == \"float\" always has a float value")
            .to_string(),
        "str" => obj.as_str().unwrap_or_default().to_string(),
        "list" => obj
            .items()
            .unwrap_or_default()
            .into_iter()
            .map(monty_ref_to_string)
            .collect(),
        _ => obj.py_repr(),
    }
}
