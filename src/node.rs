use crate::{
    Accessor, Data, GlobalBorrow, Ident, InvokeRef, MacroInvokeRef, TypeNode, ValueRef, INVOKES,
    VALUES,
};

#[derive(Debug, Clone)]
pub(crate) enum ValueNode {
    Tuple(Vec<ValueRef>),
    Str(String),
    // TODO: Add lifetime parameter
    Reference {
        is_mut: bool,
        value: ValueRef,
    },
    Dereference(ValueRef),
    Binding {
        name: Ident,
        ty: TypeNode,
    },
    DataStructure {
        name: String,
        data: Data<ValueRef>,
    },
    Invoke(InvokeRef),
    Destructure {
        parent: ValueRef,
        accessor: Accessor,
        ty: TypeNode,
    },
    MacroInvocation(MacroInvokeRef),
}

impl ValueNode {
    pub fn get_type(&self) -> TypeNode {
        match self {
            Self::Tuple(types) => {
                TypeNode::Tuple(types.iter().map(|type_ref| type_ref.get_type()).collect())
            }
            Self::Str(_) => TypeNode::PrimitiveStr,
            Self::Reference { is_mut, value } => TypeNode::Reference {
                is_mut: *is_mut,
                lifetime: None,
                inner: Box::new(value.get_type()),
            },
            Self::Binding { ty, .. } => ty.clone(),
            Self::Destructure {
                parent,
                accessor,
                ty,
            } => ty.clone(),
            Self::Invoke(invoke_ref) => {
                INVOKES.with_borrow(|invokes| invokes[invoke_ref.0].function.sig.output.clone())
            }

            node => panic!("ValueNode::get_type"),
        }
    }

    // FIXME: Consider generating invocations to std::any::type_name(), and
    // resolving generic parameters during the type and trait inference stage.
    pub fn get_type_name(&self) -> Self {
        match self {
            Self::Tuple(types) => {
                let types: String =
                    types
                        .iter()
                        .fold(String::from(""), |mut acc, v| match &v.get_type_name() {
                            Self::Str(name) => {
                                acc.push_str(name);
                                acc.push_str(", ");
                                acc
                            }
                            _ => unreachable!(),
                        });
                let types = format!("({})", types.trim_end_matches(", "));
                Self::Str(types)
            }
            Self::Str(_) => Self::Str(String::from("str")),
            Self::DataStructure { name, .. } => Self::Str(name.to_owned()),
            Self::Reference { value, .. } => value.get_type_name(),
            Self::Binding { ty, .. } => Self::Str(ty.get_name()),
            Self::Destructure {
                parent,
                accessor,
                ty,
            } => Self::Str(ty.get_name()),
            Self::Invoke(invoke_ref) => Self::Str(
                INVOKES.with_borrow(|invokes| invokes[invoke_ref.0].function.sig.output.get_name()),
            ),
            node => panic!("ValueNode::get_type_name"),
        }
    }
}

impl ValueRef {
    pub(crate) fn get_type(self) -> TypeNode {
        VALUES.with_borrow(|values| values[self.0].get_type())
    }

    pub(crate) fn get_type_name(self) -> ValueNode {
        VALUES.with_borrow(|values| values[self.0].get_type_name())
    }
}
