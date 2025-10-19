use darling::{ast, FromDeriveInput, FromField, FromVariant};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;

#[derive(Debug, FromField, Clone)]
#[darling(attributes(graphics))]
pub(crate) struct LivecodeFieldReceiver {
    pub(crate) ident: Option<syn::Ident>,
    pub(crate) kind: Option<String>,
    pub(crate) ctrl: Option<String>,
    pub(crate) texture: Option<String>,
    pub(crate) run: Option<String>,
}

// for enums
#[derive(Debug, FromVariant, Clone)]
#[darling(attributes(graphics))]
pub(crate) struct LivecodeVariantReceiver {}

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(graphics), supports(struct_named))]
pub(crate) struct LivecodeReceiver {
    ident: syn::Ident,
    data: ast::Data<LivecodeVariantReceiver, LivecodeFieldReceiver>,
    ctrl: Option<String>, // this one should be the struct name...
}

enum GraphicKind {
    Drawer,
    Pipeline,
    Graphics,
    ComputeTexture, // similar to graphics, but is a compute shader that outputs a texture
    Ref,
    DrawSrc,
}
impl GraphicKind {
    fn parse(s: &str) -> Self {
        match s {
            "drawer" => Self::Drawer,
            "pipeline" => Self::Pipeline,
            "graphics" => Self::Graphics,
            "computetexture" => Self::ComputeTexture,
            "ref" => Self::Ref,
            "draw_src" => Self::DrawSrc,
            _ => panic!("unexpected kind"),
        }
    }
}

pub fn impl_graphics_trait(ast: DeriveInput) -> TokenStream2 {
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();

    match &ast_receiver.data {
        ast::Data::Enum(_) => unreachable!("hm, graphics should be a struct"),
        ast::Data::Struct(fields) => {
            let ctrl = syn::Ident::new(
                &ast_receiver.ctrl.expect("missing ctrl with graphics type"),
                ast_receiver.ident.span(),
            );
            parse_graphics(&ast_receiver.ident, &ctrl, &fields.fields)
        }
    }
}

fn parse_graphics(
    name: &syn::Ident,
    ctrlcls: &syn::Ident,
    fields: &[LivecodeFieldReceiver],
) -> TokenStream2 {
    let mut drawers = vec![];
    let mut pipelines = vec![];
    let mut graphics = vec![];
    let mut ctrl = vec![];
    let mut to_texture = vec![];

    for f in fields {
        if let (Some(kind), Some(ident)) = (&f.kind, &f.ident) {
            let kind = GraphicKind::parse(kind);
            let ident = ident.clone();

            match kind {
                GraphicKind::Drawer => drawers.push(quote! {v.push(&self.#ident)}),
                GraphicKind::Pipeline => {
                    let result = if let Some(should_run) = &f.run {
                        let should_run_fn = syn::parse_str::<syn::Path>(should_run)
                            .expect("that's not a function!");

                        quote! {
                            if #should_run_fn(&livecoder, render_in) {
                                v.push(&self.#ident as &dyn GraphicsRenderer);
                            }
                        }
                    } else {
                        quote! {
                            v.push(&self.#ident as &dyn GraphicsRenderer);
                        }
                    };
                    pipelines.push(result);
                    ctrl.push(
                        quote! {v.extend(self.#ident.control_graphics(&livecoder).into_iter())},
                    );
                }
                GraphicKind::Graphics => {
                    graphics.push(
                        quote! {self.#ident.gpu_pipelines(render_in) as dyn GraphicsRenderer},
                    );
                    if let Some(ctrl_) = &f.ctrl {
                        let cc = syn::Ident::new(ctrl_, ident.span());
                        ctrl.push(quote! {v.extend(self.#ident.control_graphics(&livecoder.#cc).into_iter())});
                    }
                }
                GraphicKind::DrawSrc => {
                    let result = if let Some(should_run) = &f.run {
                        let should_run_fn = syn::parse_str::<syn::Path>(should_run)
                            .expect("that's not a function!");

                        quote! {
                            if !#should_run_fn(&livecoder, render_in) {
                                v.push(&self.#ident as &dyn GraphicsRenderer);
                            }
                        }
                    } else {
                        quote! {
                            v.push(&self.#ident as &dyn GraphicsRenderer);
                        }
                    };
                    pipelines.push(result);

                    if let Some(ctrl_) = &f.ctrl {
                        let ctrl_ident = syn::Ident::new(ctrl_, name.span());

                        ctrl.push(quote! {self.#ident.control_graphics(&livecoder.#ctrl_ident)});
                    }
                    if let Some(t) = &f.texture {
                        to_texture.push(quote! {(StrId::new(#t), self.#ident.texture())})
                    }
                }
                GraphicKind::Ref => {
                    if let Some(ctrl_) = &f.ctrl {
                        let ctrl_ident = syn::Ident::new(ctrl_, name.span());
                        let ident_str = ident.to_string();
                        ctrl.push(quote! {
                            v.extend(ControlGraphicsRef::new(
                                #ident_str,
                                Box::new(livecoder.#ctrl_ident.clone()),
                                Some(self.#ident.clone()),
                            ).into_iter())
                        })
                    }
                }
                GraphicKind::ComputeTexture => {
                    ctrl.push(
                        quote! {v.extend(self.#ident.control_graphics(&livecoder).into_iter())},
                    );
                }
            }
        }
    }

    quote! {

        impl SimpleGraphicsMng<#ctrlcls> for #name {
            fn drawers(&self) -> Vec<&Drawer> {
                let mut v:  Vec<&Drawer> = vec![];
                #(#drawers;)*
                v
            }
            fn gpu_pipelines<'a, 'b, 'c>(&'a self, livecoder: &'b #ctrlcls, render_in: &'c GraphicsRenderIn) -> Vec<&'a (dyn GraphicsRenderer + 'a)> {
                let mut v: Vec<&dyn GraphicsRenderer> = vec![];
                #(#pipelines;)*
                v
            }

            fn control_graphics<'a, 'b>(&'a self, livecoder: &'b #ctrlcls) -> Vec<ControlGraphicsRef> {
                let mut v: Vec<ControlGraphicsRef> = vec![];
                #(#ctrl;)*
                v
            }

            fn to_texture_map(&self) -> Vec<(StrId, Texture)> {
                vec![#(#to_texture,)*]
            }
        }

        // hrm, i don't remember why this was commented out
        impl GraphicsRenderer for #name {
            fn render(&self, device: &DeviceStateForRender) {
                // for pipeline in self.gpu_pipelines(&GraphicsRenderIn::new(true)) {
                //     pipeline.render(device);
                // }
            }
        }

    }
}
