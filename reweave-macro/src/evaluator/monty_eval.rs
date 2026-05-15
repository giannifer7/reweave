use monty::{MontyObject, MontyRun};
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
            .map(|(_, v)| MontyObject::String(v.clone()))
            .collect();
        all_args.extend(args.iter().map(|s| MontyObject::String(s.clone())));

        let runner = MontyRun::new(code.to_owned(), &format!("{macro_name}.py"), all_params)
            .map_err(|e| format!("pydef '{macro_name}': compile error: {e:?}"))?;

        let result = runner
            .run_no_limits(all_args)
            .map_err(|e| format!("pydef '{macro_name}': runtime error: {e:?}"))?;

        Ok(monty_object_to_string(result))
    }
}
pub fn monty_object_to_string(obj: MontyObject) -> String {
    match obj {
        MontyObject::None => String::new(),
        MontyObject::Bool(b) => {
            if b {
                "true".into()
            } else {
                String::new()
            }
        }
        MontyObject::Int(i) => i.to_string(),
        MontyObject::Float(f) => f.to_string(),
        MontyObject::String(s) => s,
        MontyObject::List(items) => items.into_iter().map(monty_object_to_string).collect(),
        other => format!("{other:?}"),
    }
}
