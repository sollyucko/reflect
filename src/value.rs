use crate::{
    ty::DataStructure, Accessor, Data, GlobalBorrow, GlobalPush, Struct, TupleStruct, TypeNode,
    ValueNode, ValueRef, VALUES,
};

#[derive(Debug, Clone, Copy)]
pub struct Value {
    pub(crate) index: ValueRef,
}

impl Value {
    pub fn new_tuple(values: &[Self]) -> Self {
        let node = ValueNode::Tuple(values.iter().map(|v| v.index).collect());
        Self {
            index: VALUES.index_push(node),
        }
    }

    pub fn new_reference(&self) -> Self {
        let node = ValueNode::Reference {
            is_mut: false,
            value: self.index,
        };
        Self {
            index: VALUES.index_push(node),
        }
    }

    pub fn new_reference_mut(&self) -> Self {
        let node = ValueNode::Reference {
            is_mut: true,
            value: self.index,
        };
        Self {
            index: VALUES.index_push(node),
        }
    }

    pub fn dereference(&self) -> Self {
        match self.node() {
            ValueNode::Reference { value, .. } => Self { index: value },
            other => {
                let node = ValueNode::Dereference(self.index);
                Self {
                    index: VALUES.index_push(node),
                }
            }
        }
    }

    pub fn get_type_name(&self) -> Self {
        let node = self.node().get_type_name();
        Self {
            index: VALUES.index_push(node),
        }
    }

    pub fn as_data(&self) -> Data<Self> {
        use crate::ValueNode::*;
        match self.node() {
            DataStructure { data, .. } => data.map(|value_ref| Self {
                index: value_ref.element,
            }),
            #[rustfmt::skip]
            Reference { is_mut, value } if !is_mut => {
                Self { index: value }.as_data().map(|v| v.element.new_reference())
            },
            #[rustfmt::skip]
            Reference { is_mut, value } if is_mut => {
                Self { index: value }.as_data().map(|v| v.element.new_reference_mut())
            },
            // FIXME generate match and propagate the binding
            Binding { name, ty } => ty.as_data().map(|field| {
                let node = ValueNode::Destructure {
                    parent: self.index,
                    accessor: field.accessor.clone(),
                    ty: field.element,
                };
                Self {
                    index: VALUES.index_push(node),
                }
            }),
            _ => panic!("Value::data"),
        }
    }

    /// Returns a `Value` from a `Tuple` or `TupleStruct`
    pub fn index(&self, index: usize) -> Self {
        match self.index.node() {
            ValueNode::Tuple(values) => Self {
                index: values[index],
            },
            ValueNode::Binding {
                ty: TypeNode::Tuple(types),
                ..
            } => {
                if index >= types.len() {
                    panic!("Value:get_index: Out of bounds")
                }
                let node = ValueNode::Destructure {
                    parent: self.index,
                    accessor: Accessor::Index(index),
                    ty: types[index].clone(),
                };
                Self {
                    index: VALUES.index_push(node),
                }
            }
            ValueNode::DataStructure {
                data: Data::Struct(Struct::Tuple(TupleStruct { fields, .. })),
                ..
            } => {
                let field = &fields[index];
                let node = ValueNode::Destructure {
                    parent: self.index,
                    accessor: field.accessor.clone(),
                    ty: field.element.get_type(),
                };
                Self {
                    index: VALUES.index_push(node),
                }
            }
            ValueNode::Binding {
                ty: TypeNode::DataStructure(data),
                ..
            } if is_tuple_struct(&*data) => {
                if let Data::Struct(Struct::Tuple(TupleStruct { fields, .. })) = data.data {
                    let field = &fields[index];
                    let node = ValueNode::Destructure {
                        parent: self.index,
                        accessor: field.accessor.clone(),
                        ty: field.element.clone(),
                    };
                    Self {
                        index: VALUES.index_push(node),
                    }
                } else {
                    unreachable!()
                }
            }
            node => {
                let node = ValueNode::Destructure {
                    parent: self.index,
                    accessor: Accessor::Index(index),
                    ty: TypeNode::Infer,
                };
                Self {
                    index: VALUES.index_push(node),
                }
            }
        }
    }
}

fn is_tuple_struct(data: &DataStructure) -> bool {
    if let Data::Struct(Struct::Tuple(_)) = data.data {
        true
    } else {
        false
    }
}

impl Value {
    pub(crate) fn node(self) -> ValueNode {
        self.index.node()
    }
}

impl ValueRef {
    pub(crate) fn node(self) -> ValueNode {
        VALUES.with_borrow(|values| values[self.0].clone())
    }
}
