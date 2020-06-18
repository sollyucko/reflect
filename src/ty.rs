use crate::{
    generics, Data, GenericParam, Generics, Ident, Lifetime, ParamMap, Path, Print, Struct,
    SynParamMap, TupleStruct, TypeParam, TypeParamBound,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use ref_cast::RefCast;
use std::fmt::Debug;
use syn::TypePath;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Type(pub(crate) TypeNode);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum TypeNode {
    Infer,
    Tuple(Vec<TypeNode>),
    PrimitiveStr,
    Reference {
        is_mut: bool,
        lifetime: Option<Lifetime>,
        inner: Box<TypeNode>,
    },
    Dereference(Box<TypeNode>),
    TraitObject(Vec<TypeParamBound>),
    DataStructure(Box<DataStructure>),
    Path(Path),
    TypeParam(TypeParam),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct DataStructure {
    pub name: Ident,
    pub generics: Generics,
    pub data: Data<Type>,
}

impl Type {
    pub fn new_unit() -> Self {
        Self(TypeNode::Tuple(Vec::new()))
    }

    pub fn new_tuple(types: &[Self]) -> Self {
        Self(TypeNode::Tuple(
            types.iter().cloned().map(|ty| ty.0).collect(),
        ))
    }

    pub fn new_primitive_str() -> Self {
        Self(TypeNode::PrimitiveStr)
    }

    pub fn new_reference(&self) -> Self {
        Self(TypeNode::Reference {
            is_mut: false,
            lifetime: None,
            inner: Box::new(self.0.clone()),
        })
    }

    pub fn new_reference_with_lifetime(&self, lifetime: &str, param_map: &SynParamMap) -> Self {
        let lifetime = param_map.get_lifetime(lifetime);

        Self(TypeNode::Reference {
            is_mut: false,
            lifetime: Some(lifetime),
            inner: Box::new(self.0.clone()),
        })
    }

    pub fn new_reference_mut(&self) -> Self {
        Self(TypeNode::Reference {
            is_mut: true,
            lifetime: None,
            inner: Box::new(self.0.clone()),
        })
    }

    pub fn new_reference_mut_with_lifetime(&self, lifetime: &str, param_map: &SynParamMap) -> Self {
        let lifetime = param_map.get_lifetime(lifetime);

        Self(TypeNode::Reference {
            is_mut: true,
            lifetime: Some(lifetime),
            inner: Box::new(self.0.clone()),
        })
    }

    pub fn dereference(&self) -> Self {
        match &self.0 {
            TypeNode::Reference { inner, .. } => Self((**inner).clone()),
            other => Self(TypeNode::Dereference(Box::new(other.clone()))),
        }
    }

    pub fn as_data(&self) -> Data<Self> {
        match &self.0 {
            TypeNode::DataStructure(data) => data.data.clone().map(|field| field.element),
            TypeNode::Reference {
                is_mut,
                lifetime,
                inner,
            } => Self((**inner).clone()).as_data().map(|field| {
                Self(TypeNode::Reference {
                    is_mut: *is_mut,
                    lifetime: *lifetime,
                    inner: Box::new(field.element.0),
                })
            }),
            _ => panic!("Type::data"),
        }
    }

    /// Returns a `Type` from a `Tuple` or `TupleStruct`
    pub fn index(&self, index: usize) -> Self {
        match &self.0 {
            TypeNode::Tuple(types) => Self(types[index].clone()),
            TypeNode::DataStructure(data) => {
                if let Data::Struct(Struct::Tuple(TupleStruct { fields, .. })) = &data.data {
                    fields[index].element.clone()
                } else {
                    panic!("Type::get_index: Not a TupleStruct")
                }
            }
            _ => panic!("Type::get_index: Not a Tuple"),
        }
    }

    pub fn new_trait_object(type_param_bounds: &[&str], param_map: &mut SynParamMap) -> Self {
        Self(TypeNode::TraitObject(
            type_param_bounds
                .iter()
                .map(|bound| TypeParamBound::get_type_param_bound(bound, param_map))
                .collect(),
        ))
    }

    pub fn new_type_param_from_str(type_param: &str, param_map: &mut SynParamMap) -> Self {
        if let Some(&param) = param_map.get(type_param) {
            Self(TypeNode::TypeParam(
                param
                    .type_param()
                    .expect("Type::type_param_from_str: Not a type param"),
            ))
        } else {
            panic!("Type::type_param_from_str: Not a type param")
        }
    }

    pub(crate) fn syn_to_type(ty: syn::Type, param_map: &mut SynParamMap) -> Self {
        match ty {
            syn::Type::Path(TypePath {
                //FIXME: add qself to Path
                qself: None,
                path,
            }) => {
                if let Some(ident) = path.get_ident() {
                    if let Some(&param) = param_map.get(&ident.to_string()) {
                        return Self(TypeNode::TypeParam(
                            param
                                .type_param()
                                .expect("syn_to_type: Not a type param ref"),
                        ));
                    }
                }
                Self(TypeNode::Path(Path::syn_to_path(path, param_map)))
            }

            syn::Type::Reference(reference) => {
                let inner = Box::new(Self::syn_to_type(*reference.elem, param_map).0);
                let lifetime = reference
                    .lifetime
                    .map(|lifetime| param_map.get_lifetime(&lifetime.to_string()));

                Self(TypeNode::Reference {
                    is_mut: reference.mutability.is_some(),
                    lifetime,
                    inner,
                })
            }

            syn::Type::TraitObject(type_trait_object) => Self(TypeNode::TraitObject(
                generics::syn_to_type_param_bounds(type_trait_object.bounds, param_map).collect(),
            )),

            syn::Type::Tuple(type_tuple) => {
                if type_tuple.elems.is_empty() {
                    Self::new_unit()
                } else if type_tuple.elems.len() == 1 && !type_tuple.elems.trailing_punct() {
                    // It is not a tuple. The parentheses were just used to
                    // disambiguate the type.
                    Self::syn_to_type(type_tuple.elems.into_iter().next().unwrap(), param_map)
                } else {
                    Self(TypeNode::Tuple(
                        type_tuple
                            .elems
                            .into_iter()
                            .map(|elem| Self::syn_to_type(elem, param_map).0)
                            .collect(),
                    ))
                }
            }
            _ => unimplemented!("Type::syn_to_type"),
        }
    }

    pub(crate) fn clone_with_fresh_generics(&self, param_map: &ParamMap) -> Self {
        Self(self.0.clone_with_fresh_generics(param_map))
    }
}

impl TypeNode {
    pub(crate) fn get_name(&self) -> String {
        match self {
            //FIXME: Add more TypeNode branches
            Self::Tuple(types) => {
                let types = types.iter().map(Print::ref_cast);
                quote!((#(#types),*)).to_string()
            }
            Self::PrimitiveStr => String::from("str"),
            Self::DataStructure(data) => data.name.to_string(),
            Self::Reference { inner, .. } => (&**inner).get_name(),
            Self::Path(path) => {
                let mut tokens = TokenStream::new();
                Print::ref_cast(path).to_tokens(&mut tokens);
                tokens.to_string()
            }
            Self::TypeParam(type_param) => {
                let mut tokens = TokenStream::new();
                Print::ref_cast(type_param).to_tokens(&mut tokens);
                tokens.to_string()
            }

            _ => panic!("Type::get_name"),
        }
    }

    pub(crate) fn clone_with_fresh_generics(&self, param_map: &ParamMap) -> Self {
        use super::TypeNode::*;
        match self {
            Infer => Infer,

            Tuple(types) => Tuple(
                types
                    .iter()
                    .map(|ty| ty.clone_with_fresh_generics(param_map))
                    .collect(),
            ),

            PrimitiveStr => PrimitiveStr,

            Reference {
                is_mut,
                lifetime,
                inner,
            } => Reference {
                is_mut: *is_mut,

                lifetime: lifetime.map(|lifetime| lifetime.clone_with_fresh_generics(param_map)),
                inner: Box::new(inner.clone_with_fresh_generics(param_map)),
            },

            Dereference(dereference) => {
                Dereference(Box::new(dereference.clone_with_fresh_generics(param_map)))
            }

            TraitObject(bounds) => TraitObject(
                bounds
                    .iter()
                    .map(|bound| bound.clone_with_fresh_generics(param_map))
                    .collect(),
            ),

            DataStructure { .. } => {
                unimplemented!("Type::clone_with_fresh_generics: DataStructure")
            }

            Path(path) => Path(path.clone_with_fresh_generics(param_map)),

            TypeParam(type_param) => TypeParam(
                param_map
                    .get(&GenericParam::Type(*type_param))
                    .and_then(|param| param.type_param())
                    .unwrap(),
            ),
        }
    }
}
