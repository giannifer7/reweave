use crate::evaluator::errors::{EvalError, EvalResult};
use crate::types::ASTNode;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
#[derive(Debug, Clone)]
pub struct EvalConfig {
    pub sigil: char,
    pub include_paths: Vec<PathBuf>,
    /// When true, the `%env(NAME)` builtin is permitted to read environment
    /// variables.  Disabled by default so that templates cannot silently
    /// exfiltrate secrets without the user opting in via `--allow-env`.
    pub allow_env: bool,
    /// Optional prefix prepended to `%env(NAME)` lookups.
    pub env_prefix: Option<String>,
    /// Maximum macro-call recursion depth for this evaluator run.
    pub recursion_limit: usize,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            sigil: '%',
            include_paths: vec![PathBuf::from(".")],
            allow_env: false,
            env_prefix: None,
            recursion_limit: reweave_core::MAX_RECURSION_DEPTH,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum ScriptKind {
    None,
    Python,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroBindingKind {
    Constant,
    Rebindable,
}
#[derive(Debug, Clone)]
pub struct MacroDefinition {
    pub name: String,
    pub params: Vec<String>,
    pub body: Arc<ASTNode>,
    pub script_kind: ScriptKind,
    pub binding_kind: MacroBindingKind,
    pub frozen_args: HashMap<String, String>,
}
#[derive(Debug, Default, Clone)]
pub struct ScopeFrame {
    pub variables: HashMap<String, String>,
    pub macros: HashMap<String, MacroDefinition>,
}
pub struct SourceManager {
    source_files: Vec<Vec<u8>>,
    file_names: Vec<PathBuf>,
    sources_by_path: HashMap<PathBuf, usize>,
}

impl SourceManager {
    pub fn new() -> Self {
        Self {
            source_files: Vec::new(),
            file_names: Vec::new(),
            sources_by_path: HashMap::new(),
        }
    }

    pub fn add_source_if_not_present(&mut self, file_path: PathBuf) -> Result<u32, std::io::Error> {
        let file_path = file_path.canonicalize()?;
        if let Some(&src) = self.sources_by_path.get(&file_path) {
            return Ok(src as u32);
        }
        let content = std::fs::read(file_path.clone())?;
        let src = self.add_source_bytes(content, file_path.clone());
        Ok(src)
    }

    pub fn add_source_bytes(&mut self, content: Vec<u8>, path: PathBuf) -> u32 {
        let index = self.source_files.len();
        self.source_files.push(content);
        self.file_names.push(path.clone());
        self.sources_by_path.insert(path, index);
        index as u32
    }

    pub fn get_source(&self, src: u32) -> Option<&[u8]> {
        self.source_files.get(src as usize).map(|v| v.as_slice())
    }

    pub fn num_sources(&self) -> usize {
        self.source_files.len()
    }

    pub fn source_files(&self) -> &[PathBuf] {
        &self.file_names
    }
}
pub struct EvaluatorState {
    pub config: EvalConfig,
    pub scope_stack: Vec<ScopeFrame>,
    pub open_includes: HashSet<PathBuf>,
    pub current_file: PathBuf,
    pub source_manager: SourceManager,
    pub call_depth: usize,
    /// Diagnostic warnings collected during evaluation (non-fatal).
    pub warnings: Vec<String>,
}

impl EvaluatorState {
    pub fn new(config: EvalConfig) -> Self {
        Self {
            config,
            scope_stack: vec![ScopeFrame::default()],
            open_includes: HashSet::new(),
            current_file: PathBuf::from(""),
            source_manager: SourceManager::new(),
            call_depth: 0,
            warnings: Vec::new(),
        }
    }

    pub fn push_warning(&mut self, msg: String) {
        self.warnings.push(msg);
    }

    pub fn drain_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }



    pub fn push_scope(&mut self) {
        self.scope_stack.push(ScopeFrame::default());
    }

    pub fn pop_scope(&mut self) {
        if self.scope_stack.len() > 1 {
            self.scope_stack.pop();
        }
    }

    pub fn current_scope_mut(&mut self) -> &mut ScopeFrame {
        self.scope_stack.last_mut().unwrap()
    }

    /// Set a variable with no origin tracking for computed values.
    pub fn set_variable(&mut self, name: &str, value: &str) {
        self.current_scope_mut()
            .variables
            .insert(name.into(), value.into());
    }



    /// Retrieve just the string value of a variable.
    pub fn get_variable(&self, name: &str) -> String {
        self.get_variable_opt(name).unwrap_or_default()
    }

    pub fn get_variable_opt(&self, name: &str) -> Option<String> {
        self.scope_stack
            .last()
            .and_then(|frame| frame.variables.get(name))
            .cloned()
    }


    pub fn define_macro(&mut self, mac: MacroDefinition) -> EvalResult<()> {
        if let Some(existing) = self.current_scope_mut().macros.get(&mac.name) {
            return match existing.binding_kind {
                MacroBindingKind::Constant => Err(EvalError::InvalidUsage(None, format!(
                    "cannot define macro '{}': constant binding already exists in current scope",
                    mac.name
                ))),
                MacroBindingKind::Rebindable => Err(EvalError::InvalidUsage(None, format!(
                    "cannot define macro '{}': rebindable binding already exists in current scope; use %redef",
                    mac.name
                ))),
            };
        }
        self.current_scope_mut()
            .macros
            .insert(mac.name.clone(), mac);
        Ok(())
    }

    pub fn redefine_macro(&mut self, mac: MacroDefinition) -> EvalResult<()> {
        if let Some(existing) = self.current_scope_mut().macros.get(&mac.name)
            && existing.binding_kind == MacroBindingKind::Constant
        {
            return Err(EvalError::InvalidUsage(None, format!(
                "cannot redefine macro '{}': constant binding already exists in current scope",
                mac.name
            )));
        }
        self.current_scope_mut()
            .macros
            .insert(mac.name.clone(), mac);
        Ok(())
    }

    pub fn get_macro(&self, name: &str) -> Option<MacroDefinition> {
        for frame in self.scope_stack.iter().rev() {
            if let Some(m) = frame.macros.get(name) {
                return Some(m.clone());
            }
        }
        None
    }

    pub fn get_sigil(&self) -> Vec<u8> {
        self.config.sigil.to_string().into_bytes()
    }
}
