use crate::{
    GlobalPush, MacroInvoke, Path, RuntimeType, SynParamMap, TypeNode, Value, ValueNode, MACROS,
    VALUES,
};

#[derive(Debug, Clone)]
pub struct Module {
    pub(crate) path: Path,
}

impl Module {
    pub fn root() -> Self {
        Self {
            path: Path {
                path: Vec::new(),
                global: true,
            },
        }
    }

    pub fn get_module(&self, name: &str) -> Self {
        Self {
            path: self.path.get_simple_path(name),
        }
    }

    /// Get a path by appending a path segment at the end of a module path
    pub fn get_path(&self, segment: &str, param_map: &mut SynParamMap) -> Path {
        self.path.get_path(segment, param_map)
    }

    /// Get a path type by appending a path segment at the end of a module path
    pub fn get_path_type(&self, segment: &str, param_map: &mut SynParamMap) -> TypeNode {
        self.get_path(segment, param_map).SELF()
    }

    pub fn invoke_macro(&self, name: &str, values: &[Value]) -> Value {
        let macro_path = self.path.get_simple_path(name);
        let invoke = MACROS.index_push(MacroInvoke {
            macro_path,
            args: values.iter().map(|value| value.index).collect(),
        });

        let node = ValueNode::MacroInvocation(invoke);
        Value {
            index: VALUES.index_push(node),
        }
    }
}
