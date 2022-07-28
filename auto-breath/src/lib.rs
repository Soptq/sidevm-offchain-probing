use proc_macro::*;
use quote::{quote, ToTokens};

#[proc_macro_attribute]
pub fn auto_breath(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item: syn::Item = syn::parse(input).unwrap();
    let fn_item = match &mut item {
        syn::Item::Fn(fn_item) => fn_item,
        _ => panic!("expected fn")
    };

    fn expand_expr(expr: &mut syn::Expr) {
        match expr {
            syn::Expr::Assign(_expr) => {
                expand_expr(&mut _expr.right);
            }
            syn::Expr::AssignOp(_expr) => {
                expand_expr(&mut _expr.right);
            }
            syn::Expr::Async(_async) => {
                expand_block(&mut _async.block);
            }
            syn::Expr::Block(_block) => {
                expand_block(&mut _block.block);
            }
            syn::Expr::Binary(_binary) => {
                expand_expr(&mut _binary.left);
                expand_expr(&mut _binary.right);
            }
            syn::Expr::Break(_break) => {
                if _break.expr.is_some() {
                    expand_expr(&mut _break.expr.as_mut().unwrap());
                }
            }
            syn::Expr::Closure(_closure) => {
                expand_expr(&mut _closure.body);
            }
            syn::Expr::ForLoop(_loop) => {
                expand_block(&mut _loop.body);
            }
            syn::Expr::Loop(_loop) => {
                expand_block(&mut _loop.body);
            }
            syn::Expr::If(_if) => {
                expand_block(&mut _if.then_branch);
                if _if.else_branch.is_some() {
                    expand_expr(&mut _if.else_branch.as_mut().unwrap().1);
                }
            }
            syn::Expr::Let(_let) => {
                expand_expr(&mut _let.expr);
            }
            syn::Expr::Match(_match) => {
                for arm in _match.arms.iter_mut() {
                    expand_expr(&mut arm.body);
                }
            }
            syn::Expr::While(_while) => {
                expand_block(&mut _while.body);
            }
            &mut _ => {  }
        }
    }

    fn expand_block(block: &mut syn::Block) {
        for i in (0..block.stmts.len()).rev() {
            match &mut block.stmts[i] {
                syn::Stmt::Expr(expr) => {
                    expand_expr(expr);
                }
                _ => {
                    block.stmts.insert(i, syn::parse(
                        quote!(
                            sidevm::time::sleep(Duration::from_millis(0)).await;
                        ).into()
                    ).unwrap());
                }
            }
        }
    }

    expand_block(&mut fn_item.block);
    item.into_token_stream().into()
}