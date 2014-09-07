#![crate_name="autobind"]
#![crate_type="dylib"]

#![feature(quote, phase, plugin_registrar, macro_rules)]

extern crate syntax;
extern crate rustc;

#[phase(plugin, link)]
extern crate log;

use std::ascii::AsciiExt;
use std::gc::{Gc, GC};

use syntax::abi;
use syntax::ast;
use syntax::codemap;
use syntax::owned_slice::OwnedSlice;
use syntax::ext::base::{ExtCtxt, ItemDecorator};
use syntax::parse::token::intern;

use rustc::plugin::Registry;


#[plugin_registrar]
#[doc(hidden)]
pub fn plugin_registrar(registrar: &mut Registry) {
    registrar.register_syntax_extension(intern("autobind"), ItemDecorator(autobind));
}

fn autobind(cx: &mut ExtCtxt,
            sp: codemap::Span,
            _attr: Gc<ast::MetaItem>,
            it: Gc<ast::Item>,
            add: |it: Gc<ast::Item>|)
{
    let err_span = codemap::mk_sp(sp.lo, it.span.hi);

    // Verify that the input item is of the right type.
    let methods = match it.node {
        ast::ItemImpl(_, ref tr, _, ref methods) => {
            // We can't autobind an impl of a trait.
            if tr.is_some() {
                cx.span_err(err_span, "#[autobind] cannot be applied to impls of traits");
                return
            }

            // Good!  We have some methods.
            methods
        },
        _ => {
            cx.span_err(err_span, "#[autobind] can only be applied to impl blocks");
            return
        },
    };

    let tyname = it.ident.as_str();

    // For each method, potentially autobind them.
    for m in methods.iter() {
        let ast::MethodImplItem(ref m) = *m;

        match m.node {
            ast::MethDecl(ref id, ref _gen, ref _abi, ref selfty, ref _style,
                          ref decl, ref _block, ref vis) => {
                if *vis == ast::Public {
                    debug!("Found public method: {}", id.as_str());

                    process_method(cx,
                                   tyname,
                                   &m.span,
                                   id,
                                   selfty,
                                   decl,
                                   |x| add(x)
                                  );
                }
            },
            _ => {},
        };
    }
}

fn process_method(cx: &mut ExtCtxt,
                  tyname: &str,
                  sp: &codemap::Span,
                  id: &ast::Ident,
                  selfty: &ast::ExplicitSelf,
                  decl: &ast::P<ast::FnDecl>,
                  add: |it: Gc<ast::Item>|)
{
    let mut skip_arg;

    match selfty.node {
        ast::SelfStatic => {
            debug!("* static method");
            skip_arg = 0u;
        },
        ast::SelfRegion(None, ast::MutImmutable, _) => {
            debug!("* by-ref immutable self");
            skip_arg = 1;
        },
        ast::SelfRegion(None, ast::MutMutable, _) => {
            debug!("* by-ref mutable self");
            skip_arg = 1;
        },

        // Various unsupported things follow.
        ast::SelfRegion(Some(_), _, _) => {
            debug!("* by-ref self with region");

            cx.span_warn(sp.clone(),
                "autobind: cannot generate bindings for method with a lifetime on `self`");
            return
        },
        ast::SelfValue(_) => {
            debug!("* by-value self: {}", id);

            cx.span_warn(sp.clone(),
                "autobind: cannot generate bindings for method with by-value `self`");
            return
        },
        ast::SelfExplicit(_, _) => {
            debug!("* explicit self");

            cx.span_warn(sp.clone(),
                "autobind: cannot generate bindings for method with explicit `self`");
            return
        },
    };

    debug!("* method has {} arguments", decl.inputs.len() - skip_arg);

    // Bind each argument.
    let mut bindings = vec![];
    for (i, arg) in decl.inputs.iter().skip(skip_arg).enumerate() {
        match get_path_for(cx, i, &arg.ty) {
            Some(b) => bindings.push(b),
            None    => return,
        };
    }

    // This is the top-level function that we generate.
    let binding_func = ast::ItemFn(
        // Declaration.
        box(GC) ast::FnDecl {
            inputs: vec![],

            output: box(GC) ast::Ty {
                // Return type
                node: ast::TyNil,

                id: ast::DUMMY_NODE_ID,
                span: codemap::DUMMY_SP,
            },

            cf: ast::Return,
            variadic: false,
        },

        // Function style
        ast::UnsafeFn,

        // Function ABI
        abi::C,

        // Generics (none)
        ast::Generics {
            lifetimes: vec![],
            ty_params: OwnedSlice::empty(),
            where_clause: ast::WhereClause {
                id: ast::DUMMY_NODE_ID,
                predicates: vec![],
            },
        },

        // The block.
        // TODO: check on this
        box(GC) ast::Block {
            view_items: vec![],
            stmts: vec![],
            expr: None,
            id: ast::DUMMY_NODE_ID,
            rules: ast::DefaultBlock,
            span: codemap::DUMMY_SP,
        },
    );

    // Generate function name.
    let fn_name = tyname.to_ascii_lower().append("_").append(id.as_str());

    // Wrap into a real Item with the function name.
    let it = box(GC) ast::Item {
        ident: ast::Ident::new(intern(fn_name.as_slice())),
        attrs: vec![],
        id: ast::DUMMY_NODE_ID,
        node: binding_func,
        vis: ast::Public,
        span: codemap::DUMMY_SP,
    };

    // Add it!
    add(it)
}

// Generate marshal code and types for a Rust type.
fn get_path_for(cx: &mut ExtCtxt,
                arg_num: uint,
                ty: &ast::P<ast::Ty>) -> Option<CBinding>
{
    let typath = match ty.node {
        ast::TyPath(ref path, ref bounds, ref _id) => {
            if bounds.is_some() {
                cx.span_warn(ty.span,
                    "autobind: currently can't handle bounds on a path");
                return None;
            }

            path
        },

        _ => {
            cx.span_warn(ty.span,
                "autobind: unknown type for binding");
            return None;
        }
    };

    let init = if typath.global {
        String::from_str("::")
    } else {
        String::new()
    };

    if !typath.segments.iter().all(|seg| seg.lifetimes.len() == 0) {
        cx.span_warn(ty.span,
            "autobind: currently can't handle lifetimes in a path");
        return None;
    }
    if !typath.segments.iter().all(|seg| seg.types.as_slice().len() == 0) {
        cx.span_warn(ty.span,
            "autobind: currently can't handle type parameters in a path");
        return None;
    }

    let path_str = typath.segments.
        iter().
        fold(init, |accum, seg| accum.append(seg.identifier.as_str()));

    debug!("  * arg {} type is: {}", arg_num, path_str);

    // Generate the code to match.
    let binding = match path_str {
        /* "int" => {
            CBinding {
                expr: quote_expr!( arg as c_int  ),
                c_type: String::from_str("c_int"),
            }
        }, */
        p => {
            cx.span_warn(ty.span,
                format!("autobind: unsupported type for binding: {}", p).as_slice());
            return None;
        },
    };

    Some(binding)
}

struct CBinding {
    // Expression that binds C type to the Rust type.
    pub expr: ast::Expr,

    // C type as a string.
    pub c_type: String,
}
