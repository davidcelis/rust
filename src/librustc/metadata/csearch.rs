// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// Searching for information from the cstore

use metadata::common::*;
use metadata::cstore;
use metadata::decoder;
use middle::lang_items;
use middle::ty;

use rbml;
use rbml::reader;
use std::rc::Rc;
use syntax::ast;
use syntax::ast_map;
use syntax::attr;
use syntax::attr::AttrMetaMethods;
use syntax::diagnostic::expect;
use syntax::parse::token;

use std::collections::hash_map::HashMap;

#[derive(Copy, Clone)]
pub struct MethodInfo {
    pub name: ast::Name,
    pub def_id: ast::DefId,
    pub vis: ast::Visibility,
}

pub fn get_symbol(cstore: &cstore::CStore, def: ast::DefId) -> String {
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_symbol(cdata.data(), def.node)
}

/// Iterates over all the language items in the given crate.
pub fn each_lang_item<F>(cstore: &cstore::CStore,
                         cnum: ast::CrateNum,
                         f: F)
                         -> bool where
    F: FnMut(ast::NodeId, usize) -> bool,
{
    let crate_data = cstore.get_crate_data(cnum);
    decoder::each_lang_item(&*crate_data, f)
}

/// Iterates over each child of the given item.
pub fn each_child_of_item<F>(cstore: &cstore::CStore,
                             def_id: ast::DefId,
                             callback: F) where
    F: FnMut(decoder::DefLike, ast::Name, ast::Visibility),
{
    let crate_data = cstore.get_crate_data(def_id.krate);
    let get_crate_data = |cnum| {
        cstore.get_crate_data(cnum)
    };
    decoder::each_child_of_item(cstore.intr.clone(),
                                &*crate_data,
                                def_id.node,
                                get_crate_data,
                                callback)
}

/// Iterates over each top-level crate item.
pub fn each_top_level_item_of_crate<F>(cstore: &cstore::CStore,
                                       cnum: ast::CrateNum,
                                       callback: F) where
    F: FnMut(decoder::DefLike, ast::Name, ast::Visibility),
{
    let crate_data = cstore.get_crate_data(cnum);
    let get_crate_data = |cnum| {
        cstore.get_crate_data(cnum)
    };
    decoder::each_top_level_item_of_crate(cstore.intr.clone(),
                                          &*crate_data,
                                          get_crate_data,
                                          callback)
}

pub fn get_item_path(tcx: &ty::ctxt, def: ast::DefId) -> Vec<ast_map::PathElem> {
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(def.krate);
    let path = decoder::get_item_path(&*cdata, def.node);

    // FIXME #1920: This path is not always correct if the crate is not linked
    // into the root namespace.
    let mut r = vec![ast_map::PathMod(token::intern(&cdata.name))];
    r.push_all(&path);
    r
}

pub enum FoundAst<'ast> {
    Found(&'ast ast::InlinedItem),
    FoundParent(ast::DefId, &'ast ast::InlinedItem),
    NotFound,
}

// Finds the AST for this item in the crate metadata, if any.  If the item was
// not marked for inlining, then the AST will not be present and hence none
// will be returned.
pub fn maybe_get_item_ast<'tcx>(tcx: &ty::ctxt<'tcx>, def: ast::DefId,
                                decode_inlined_item: decoder::DecodeInlinedItem)
                                -> FoundAst<'tcx> {
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(def.krate);
    decoder::maybe_get_item_ast(&*cdata, tcx, def.node, decode_inlined_item)
}

pub fn get_enum_variants<'tcx>(tcx: &ty::ctxt<'tcx>, def: ast::DefId)
                               -> Vec<Rc<ty::VariantInfo<'tcx>>> {
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_enum_variants(cstore.intr.clone(), &*cdata, def.node, tcx)
}

/// Returns information about the given implementation.
pub fn get_impl_items(cstore: &cstore::CStore, impl_def_id: ast::DefId)
                      -> Vec<ty::ImplOrTraitItemId> {
    let cdata = cstore.get_crate_data(impl_def_id.krate);
    decoder::get_impl_items(&*cdata, impl_def_id.node)
}

pub fn get_impl_or_trait_item<'tcx>(tcx: &ty::ctxt<'tcx>, def: ast::DefId)
                                    -> ty::ImplOrTraitItem<'tcx> {
    let cdata = tcx.sess.cstore.get_crate_data(def.krate);
    decoder::get_impl_or_trait_item(tcx.sess.cstore.intr.clone(),
                                    &*cdata,
                                    def.node,
                                    tcx)
}

pub fn get_trait_name(cstore: &cstore::CStore, def: ast::DefId) -> ast::Name {
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_trait_name(cstore.intr.clone(),
                            &*cdata,
                            def.node)
}

pub fn is_static_method(cstore: &cstore::CStore, def: ast::DefId) -> bool {
    let cdata = cstore.get_crate_data(def.krate);
    decoder::is_static_method(&*cdata, def.node)
}

pub fn get_trait_item_def_ids(cstore: &cstore::CStore, def: ast::DefId)
                              -> Vec<ty::ImplOrTraitItemId> {
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_trait_item_def_ids(&*cdata, def.node)
}

pub fn get_item_variances(cstore: &cstore::CStore,
                          def: ast::DefId) -> ty::ItemVariances {
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_item_variances(&*cdata, def.node)
}

pub fn get_provided_trait_methods<'tcx>(tcx: &ty::ctxt<'tcx>,
                                        def: ast::DefId)
                                        -> Vec<Rc<ty::Method<'tcx>>> {
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_provided_trait_methods(cstore.intr.clone(), &*cdata, def.node, tcx)
}

pub fn get_associated_consts<'tcx>(tcx: &ty::ctxt<'tcx>, def: ast::DefId)
                                   -> Vec<Rc<ty::AssociatedConst<'tcx>>> {
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_associated_consts(cstore.intr.clone(), &*cdata, def.node, tcx)
}

pub fn get_type_name_if_impl(cstore: &cstore::CStore, def: ast::DefId)
                          -> Option<ast::Name> {
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_type_name_if_impl(&*cdata, def.node)
}

pub fn get_methods_if_impl(cstore: &cstore::CStore,
                                  def: ast::DefId)
                               -> Option<Vec<MethodInfo> > {
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_methods_if_impl(cstore.intr.clone(), &*cdata, def.node)
}

pub fn get_item_attrs(cstore: &cstore::CStore,
                      def_id: ast::DefId)
                      -> Vec<ast::Attribute> {
    let cdata = cstore.get_crate_data(def_id.krate);
    decoder::get_item_attrs(&*cdata, def_id.node)
}

pub fn get_struct_fields(cstore: &cstore::CStore,
                         def: ast::DefId)
                      -> Vec<ty::field_ty> {
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_struct_fields(cstore.intr.clone(), &*cdata, def.node)
}

pub fn get_struct_field_attrs(cstore: &cstore::CStore, def: ast::DefId) -> HashMap<ast::NodeId,
        Vec<ast::Attribute>> {
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_struct_field_attrs(&*cdata)
}

pub fn get_type<'tcx>(tcx: &ty::ctxt<'tcx>,
                      def: ast::DefId)
                      -> ty::TypeScheme<'tcx> {
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_type(&*cdata, def.node, tcx)
}

pub fn get_trait_def<'tcx>(tcx: &ty::ctxt<'tcx>, def: ast::DefId) -> ty::TraitDef<'tcx> {
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_trait_def(&*cdata, def.node, tcx)
}

pub fn get_predicates<'tcx>(tcx: &ty::ctxt<'tcx>, def: ast::DefId)
                            -> ty::GenericPredicates<'tcx>
{
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_predicates(&*cdata, def.node, tcx)
}

pub fn get_super_predicates<'tcx>(tcx: &ty::ctxt<'tcx>, def: ast::DefId)
                                  -> ty::GenericPredicates<'tcx>
{
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_super_predicates(&*cdata, def.node, tcx)
}

pub fn get_field_type<'tcx>(tcx: &ty::ctxt<'tcx>, class_id: ast::DefId,
                            def: ast::DefId) -> ty::TypeScheme<'tcx> {
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(class_id.krate);
    let all_items = reader::get_doc(rbml::Doc::new(cdata.data()), tag_items);
    let class_doc = expect(tcx.sess.diagnostic(),
                           decoder::maybe_find_item(class_id.node, all_items),
                           || {
        (format!("get_field_type: class ID {:?} not found",
                 class_id)).to_string()
    });
    let the_field = expect(tcx.sess.diagnostic(),
        decoder::maybe_find_item(def.node, class_doc),
        || {
            (format!("get_field_type: in class {:?}, field ID {:?} not found",
                    class_id,
                    def)).to_string()
        });
    let ty = decoder::item_type(def, the_field, tcx, &*cdata);
    ty::TypeScheme {
        generics: ty::Generics::empty(),
        ty: ty,
    }
}

pub fn get_impl_polarity<'tcx>(tcx: &ty::ctxt<'tcx>,
                               def: ast::DefId)
                               -> Option<ast::ImplPolarity>
{
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_impl_polarity(&*cdata, def.node)
}

pub fn get_custom_coerce_unsized_kind<'tcx>(tcx: &ty::ctxt<'tcx>,
                                            def: ast::DefId)
                                            -> Option<ty::CustomCoerceUnsized> {
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_custom_coerce_unsized_kind(&*cdata, def.node)
}

// Given a def_id for an impl, return the trait it implements,
// if there is one.
pub fn get_impl_trait<'tcx>(tcx: &ty::ctxt<'tcx>,
                            def: ast::DefId)
                            -> Option<ty::TraitRef<'tcx>> {
    let cstore = &tcx.sess.cstore;
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_impl_trait(&*cdata, def.node, tcx)
}

pub fn get_native_libraries(cstore: &cstore::CStore, crate_num: ast::CrateNum)
                            -> Vec<(cstore::NativeLibraryKind, String)> {
    let cdata = cstore.get_crate_data(crate_num);
    decoder::get_native_libraries(&*cdata)
}

pub fn each_inherent_implementation_for_type<F>(cstore: &cstore::CStore,
                                                def_id: ast::DefId,
                                                callback: F) where
    F: FnMut(ast::DefId),
{
    let cdata = cstore.get_crate_data(def_id.krate);
    decoder::each_inherent_implementation_for_type(&*cdata, def_id.node, callback)
}

pub fn each_implementation_for_trait<F>(cstore: &cstore::CStore,
                                        def_id: ast::DefId,
                                        mut callback: F) where
    F: FnMut(ast::DefId),
{
    cstore.iter_crate_data(|_, cdata| {
        decoder::each_implementation_for_trait(cdata, def_id, &mut callback)
    })
}

/// If the given def ID describes an item belonging to a trait (either a
/// default method or an implementation of a trait method), returns the ID of
/// the trait that the method belongs to. Otherwise, returns `None`.
pub fn get_trait_of_item(cstore: &cstore::CStore,
                         def_id: ast::DefId,
                         tcx: &ty::ctxt)
                         -> Option<ast::DefId> {
    let cdata = cstore.get_crate_data(def_id.krate);
    decoder::get_trait_of_item(&*cdata, def_id.node, tcx)
}

pub fn get_tuple_struct_definition_if_ctor(cstore: &cstore::CStore,
                                           def_id: ast::DefId)
    -> Option<ast::DefId>
{
    let cdata = cstore.get_crate_data(def_id.krate);
    decoder::get_tuple_struct_definition_if_ctor(&*cdata, def_id.node)
}

pub fn get_dylib_dependency_formats(cstore: &cstore::CStore,
                                    cnum: ast::CrateNum)
    -> Vec<(ast::CrateNum, cstore::LinkagePreference)>
{
    let cdata = cstore.get_crate_data(cnum);
    decoder::get_dylib_dependency_formats(&*cdata)
}

pub fn get_missing_lang_items(cstore: &cstore::CStore, cnum: ast::CrateNum)
    -> Vec<lang_items::LangItem>
{
    let cdata = cstore.get_crate_data(cnum);
    decoder::get_missing_lang_items(&*cdata)
}

pub fn get_method_arg_names(cstore: &cstore::CStore, did: ast::DefId)
    -> Vec<String>
{
    let cdata = cstore.get_crate_data(did.krate);
    decoder::get_method_arg_names(&*cdata, did.node)
}

pub fn get_reachable_extern_fns(cstore: &cstore::CStore, cnum: ast::CrateNum)
    -> Vec<ast::DefId>
{
    let cdata = cstore.get_crate_data(cnum);
    decoder::get_reachable_extern_fns(&*cdata)
}

pub fn is_typedef(cstore: &cstore::CStore, did: ast::DefId) -> bool {
    let cdata = cstore.get_crate_data(did.krate);
    decoder::is_typedef(&*cdata, did.node)
}

pub fn is_const_fn(cstore: &cstore::CStore, did: ast::DefId) -> bool {
    let cdata = cstore.get_crate_data(did.krate);
    decoder::is_const_fn(&*cdata, did.node)
}

pub fn is_impl(cstore: &cstore::CStore, did: ast::DefId) -> bool {
    let cdata = cstore.get_crate_data(did.krate);
    decoder::is_impl(&*cdata, did.node)
}

pub fn get_stability(cstore: &cstore::CStore,
                     def: ast::DefId)
                     -> Option<attr::Stability> {
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_stability(&*cdata, def.node)
}

pub fn is_staged_api(cstore: &cstore::CStore, krate: ast::CrateNum) -> bool {
    let cdata = cstore.get_crate_data(krate);
    let attrs = decoder::get_crate_attributes(cdata.data());
    for attr in &attrs {
        if &attr.name()[..] == "staged_api" {
            match attr.node.value.node { ast::MetaWord(_) => return true, _ => (/*pass*/) }
        }
    }

    return false;
}

pub fn get_repr_attrs(cstore: &cstore::CStore, def: ast::DefId)
                      -> Vec<attr::ReprAttr> {
    let cdata = cstore.get_crate_data(def.krate);
    decoder::get_repr_attrs(&*cdata, def.node)
}

pub fn is_defaulted_trait(cstore: &cstore::CStore, trait_def_id: ast::DefId) -> bool {
    let cdata = cstore.get_crate_data(trait_def_id.krate);
    decoder::is_defaulted_trait(&*cdata, trait_def_id.node)
}

pub fn is_default_impl(cstore: &cstore::CStore, impl_did: ast::DefId) -> bool {
    let cdata = cstore.get_crate_data(impl_did.krate);
    decoder::is_default_impl(&*cdata, impl_did.node)
}
